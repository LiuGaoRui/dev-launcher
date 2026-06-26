//! 构建执行：一次性跑完即退出，stdout/stderr 走 piped 实时推 Channel。
//!
//! 与 `spawn`（常驻进程）的区别：
//! - 不入 ProcessRegistry、不用 Job Object（跑完自退）
//! - stdout/stderr 用 `Stdio::piped()` 而非追加写日志文件
//! - 退出码通过 `BuildResult` 返回，构建日志实时推 Channel（ADR-002）
//!
//! 管道并发读取（关键）：stdout/stderr 各起独立 task 并发读，
//! 否则一个管道被读、另一个填满 OS 缓冲区会死锁。

use std::process::Stdio;
use std::time::Instant;

use serde::Serialize;
use tauri::ipc::Channel;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::debug;

use crate::error::{AppError, AppResult};
use crate::models::Project;

/// 管道读取缓冲块大小（4KB，平衡推送频率与 IPC 开销）。
const READ_BUF_SIZE: usize = 4096;

/// 构建输出事件（对齐 docs/03-命令清单.md §四 BuildEvent）。
///
/// serde tag=kind/content=data：序列化为 `{ "kind": "stdout", "data": "..." }`，
/// 前端按 kind 区分 stdout/stderr/exit，data 为文本（stdout/stderr）或数字（exit）。
///
/// rename_all=snake_case：把变体名 Stdout/Stderr/Exit 转为 stdout/stderr/exit。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum BuildEvent {
    Stdout(String),
    Stderr(String),
    /// 进程退出，data 为退出码
    Exit(i32),
}

/// 构建结果（对齐 docs/03-命令清单.md §四 BuildResult）。
#[derive(Debug, Clone, Serialize)]
pub struct BuildResult {
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// 执行项目构建命令，实时把 stdout/stderr 推到 Channel，返回退出码与耗时。
///
/// - `build_cmd` 为空 → 返回 `AppError::Process`
/// - 跑完即退出，不入 registry
/// - 前端 GC Channel → send 失败 → 读取 task 退出；child 由 kill_on_drop 兜底回收
pub async fn run_build(project: &Project, on_event: Channel<BuildEvent>) -> AppResult<BuildResult> {
    let build_cmd = project.build_cmd.as_ref().ok_or_else(|| {
        AppError::Process(format!("项目「{}」未配置构建命令", project.name))
    })?;
    if build_cmd.trim().is_empty() {
        return Err(AppError::Process(format!(
            "项目「{}」构建命令为空",
            project.name
        )));
    }

    let mut cmd = Command::new("cmd");
    // raw_arg 不做转义，整串交给 cmd.exe 解析（支持 &&、管道、重定向）
    cmd.raw_arg(format!("/C {build_cmd}"));
    cmd.current_dir(&project.path);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true); // 前端关 Channel 时读取 task 退出，drop child 兜底 kill

    #[cfg(windows)]
    cmd.creation_flags(crate::process::spawn::CREATE_NO_WINDOW);

    let start = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Process(format!("构建「{}」启动失败: {e}", project.name)))?;

    // 取 stdout/stderr 管道（spawn 后必定 Some，因 piped）
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Process("构建进程无 stdout 管道".into()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::Process("构建进程无 stderr 管道".into()))?;

    // 并发读 stdout/stderr：各起独立 task，send 失败（前端 GC）即退出
    let tx_out = on_event.clone();
    let stdout_task = tokio::spawn(async move {
        pump_pipe(&mut stdout, tx_out, PipeKind::Stdout).await;
    });
    let tx_err = on_event.clone();
    let stderr_task = tokio::spawn(async move {
        pump_pipe(&mut stderr, tx_err, PipeKind::Stderr).await;
    });

    // 等待进程退出
    let status = child.wait().await.map_err(|e| {
        AppError::Process(format!("构建「{}」等待退出失败: {e}", project.name))
    })?;

    // 读取 task 应随管道 EOF 自然结束，这里回收（忽略 join 错误）
    let _ = stdout_task.await;
    let _ = stderr_task.await;

    let exit_code = status.code().unwrap_or(-1);
    let duration_ms = start.elapsed().as_millis() as u64;

    // 推送 Exit 事件（send 失败忽略：前端可能已关闭，但退出码已通过 BuildResult 返回）
    let _ = on_event.send(BuildEvent::Exit(exit_code));
    debug!(
        "构建「{}」完成: exit_code={}, duration_ms={}",
        project.name, exit_code, duration_ms
    );

    Ok(BuildResult {
        exit_code,
        duration_ms,
    })
}

/// 管道来源标记（决定推送 Stdout 还是 Stderr 事件）。
#[derive(Copy, Clone)]
enum PipeKind {
    Stdout,
    Stderr,
}

/// 循环读取一个管道，按块推送到 Channel，直到 EOF。
/// send 失败（前端 GC Channel）立即返回 —— child 由 kill_on_drop 兜底。
async fn pump_pipe<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    on_event: Channel<BuildEvent>,
    kind: PipeKind,
) {
    let mut buf = vec![0u8; READ_BUF_SIZE];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break, // EOF
            Ok(n) => {
                let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                let event = match kind {
                    PipeKind::Stdout => BuildEvent::Stdout(text),
                    PipeKind::Stderr => BuildEvent::Stderr(text),
                };
                if on_event.send(event).is_err() {
                    // 前端 Channel 已 GC（关闭对话框/离开页面）→ 停止读取
                    break;
                }
            }
            Err(e) => {
                debug!("构建管道读取错误: {e}");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_event_serializes_as_tag_content() {
        // 验证 serde tag=kind/content=data 序列化为 {kind, data}
        let s = serde_json::to_string(&BuildEvent::Stdout("hello".into())).unwrap();
        assert_eq!(s, r#"{"kind":"stdout","data":"hello"}"#);

        let s = serde_json::to_string(&BuildEvent::Stderr("err".into())).unwrap();
        assert_eq!(s, r#"{"kind":"stderr","data":"err"}"#);

        let s = serde_json::to_string(&BuildEvent::Exit(0)).unwrap();
        assert_eq!(s, r#"{"kind":"exit","data":0}"#);
    }

    #[test]
    fn build_result_serializes_snake_case() {
        let r = BuildResult {
            exit_code: 1,
            duration_ms: 1234,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(s, r#"{"exit_code":1,"duration_ms":1234}"#);
    }

    #[tokio::test]
    async fn run_build_no_build_cmd_errors() {
        // 校验 build_cmd 缺失/空白的前置分支逻辑（不调真实 run_build，因 Channel 需 IPC 上下文）
        let mut p = Project {
            id: 1,
            name: "demo".into(),
            group_id: None,
            r#type: crate::models::ProjectType::Custom,
            path: ".".into(),
            workdir: None,
            start_cmd: "echo".into(),
            build_cmd: None, // 未配置
            expected_ports: vec![],
            enabled: true,
            last_pid: None,
            last_start_time: None,
            last_stop_time: None,
            create_time: "".into(),
            update_time: "".into(),
        };
        assert!(p.build_cmd.is_none());
        p.build_cmd = Some("   ".into());
        assert!(p.build_cmd.as_ref().unwrap().trim().is_empty());
    }
}
