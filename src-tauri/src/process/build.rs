//! 构建执行：一次性跑完即退出，stdout/stderr 重定向到构建日志文件。
//!
//! 与 `spawn`（常驻进程）的区别：
//! - 不入 ProcessRegistry、不用 Job Object（跑完自退）
//! - stdout/stderr 重定向到 `build.log`（**构建开始前 truncate**，只含本次输出），
//!   由 log 模块的 tail task 实时推送给前端（与启动日志同一套机制）
//! - 退出码通过 `BuildResult` 返回，写入全局 builds 状态表供前端轮询
//!
//! 调用约定：`commands::process::build_project` 用 tokio::spawn 在后台跑本函数，
//! 命令本身立即返回；前端轮询 `get_build_status` 获取 running/exit_code。
//! 构建卡死可经 `stop_build` 杀进程树取消；超过 `BUILD_TIMEOUT` 自动画树兜底。

use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::process::Command;
use tokio::sync::oneshot;
use tracing::debug;

use crate::error::{AppError, AppResult};
use crate::models::Project;

/// 构建结果：退出码与耗时。写入全局 builds 状态表供前端轮询。
#[derive(Debug, Clone, Serialize)]
pub struct BuildResult {
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// 构建超时上限：超时强制杀进程树并以失败收场。
///
/// 兜底场景：构建命令本体已成功但进程树不退出（如 Next.js 16 Turbopack
/// 打印完路由表后 worker 进程不退出），避免前端永远显示构建中。
/// 取 1 小时以覆盖大型 Java 项目的全量打包。
const BUILD_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// 执行项目构建命令：stdout/stderr 重定向到 build.log，跑完返回退出码与耗时。
///
/// - `build_cmd` 为空 → 写错误行到 build.log 后返回 `AppError::Process`
/// - 构建开始前 truncate `build.log`（保证只含本次输出）
/// - 跑完即退出，不入 registry
/// - `pid_tx`：spawn 成功即上报进程树根 pid，供 stop_build 杀树；
///   接收方已放弃（stop 来得比 spawn 还早）时发送失败可安全忽略
/// - 超过 [`BUILD_TIMEOUT`] 未退出 → taskkill 杀树并返回超时错误
pub async fn run_build(
    project: &Project,
    logs_root: &Path,
    pid_tx: oneshot::Sender<u32>,
) -> AppResult<BuildResult> {
    // 构建日志路径 + 构建前 truncate（保证只含本次输出）
    let log_path = crate::logs::paths::log_path_of(
        logs_root,
        project.id,
        crate::logs::paths::LogType::Build,
    );
    // 确保目录存在（首次构建时目录可能还没建）
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::process::spawn::truncate_log(&log_path);

    // 校验构建命令；失败时把原因写进日志，避免用户打开日志只看到空白
    let build_cmd = match project.build_cmd.as_deref() {
        Some(c) if !c.trim().is_empty() => c,
        Some(_) => {
            crate::process::spawn::append_log_line(&log_path, "# [项目管理器] 构建命令为空");
            return Err(AppError::Process(format!(
                "项目「{}」构建命令为空",
                project.name
            )));
        }
        None => {
            crate::process::spawn::append_log_line(&log_path, "# [项目管理器] 未配置构建命令");
            return Err(AppError::Process(format!(
                "项目「{}」未配置构建命令",
                project.name
            )));
        }
    };

    // 打开两个 append 句柄（stdout/stderr 各一），OS 负责交错写入
    let (stdout, stderr) = crate::process::spawn::open_log_stdio(&log_path)?;

    let mut cmd = Command::new("cmd");
    // raw_arg 不做转义，整串交给 cmd.exe 解析（支持 &&、管道、重定向）
    cmd.raw_arg(format!("/C {build_cmd}"));
    // cwd 必须与 spawn_command（start_cmd 执行）一致：detect_service 生成命令时
    // 相对路径以 workdir（= scan_root）为基准，此处也用 workdir，否则 -f/jar 路径错位。
    cmd.current_dir(crate::process::spawn::resolve_cwd(project));
    cmd.stdin(Stdio::null());
    cmd.stdout(stdout);
    cmd.stderr(stderr);
    cmd.kill_on_drop(true); // 后台构建异常时兜底回收子进程

    #[cfg(windows)]
    cmd.creation_flags(crate::process::spawn::CREATE_NO_WINDOW);

    let start = Instant::now();
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            crate::process::spawn::append_log_line(
                &log_path,
                &format!("# [项目管理器] 构建进程启动失败: {e}"),
            );
            return Err(AppError::Process(format!(
                "构建「{}」启动失败: {e}",
                project.name
            )));
        }
    };
    let pid = child
        .id()
        .ok_or_else(|| AppError::Process(format!("构建「{}」无法获取 PID", project.name)))?;
    // spawn 成功即上报 pid（接收方已放弃时忽略发送失败）
    let _ = pid_tx.send(pid);

    // 等待退出，超时杀树兜底
    let status = match tokio::time::timeout(BUILD_TIMEOUT, child.wait()).await {
        Ok(res) => res.map_err(|e| {
            AppError::Process(format!("构建「{}」等待退出失败: {e}", project.name))
        })?,
        Err(_) => {
            // 超时：强制杀整棵进程树（含 cmd/pnpm/node worker 等），回收后报错
            let _ = crate::process::kill::kill_process_tree(pid).await;
            let _ = child.wait().await;
            let mins = BUILD_TIMEOUT.as_secs() / 60;
            crate::process::spawn::append_log_line(
                &log_path,
                &format!("# [项目管理器] 构建超时（{mins} 分钟），已强制终止进程树"),
            );
            return Err(AppError::Process(format!(
                "构建「{}」超时（{mins} 分钟），已强制终止进程树",
                project.name
            )));
        }
    };

    let exit_code = status.code().unwrap_or(-1);
    let duration_ms = start.elapsed().as_millis() as u64;
    // 收场标记行：构建命令自身打完就没了（如 next build 止于路由表图例），
    // 追加一行让日志末尾自解释（结果 + 耗时），tail 订阅实时可见
    crate::process::spawn::append_log_line(
        &log_path,
        &format!(
            "# [项目管理器] 构建结束：exit_code={exit_code}，耗时 {:.1}s",
            duration_ms as f64 / 1000.0
        ),
    );
    debug!(
        "构建「{}」完成: exit_code={}, duration_ms={}",
        project.name, exit_code, duration_ms
    );

    Ok(BuildResult {
        exit_code,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_result_serializes_snake_case() {
        let r = BuildResult {
            exit_code: 1,
            duration_ms: 1234,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert_eq!(s, r#"{"exit_code":1,"duration_ms":1234}"#);
    }
}
