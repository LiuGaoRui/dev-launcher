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

use std::path::Path;
use std::process::Stdio;
use std::time::Instant;

use serde::Serialize;
use tokio::process::Command;
use tracing::debug;

use crate::error::{AppError, AppResult};
use crate::models::Project;

/// 构建结果：退出码与耗时。写入全局 builds 状态表供前端轮询。
#[derive(Debug, Clone, Serialize)]
pub struct BuildResult {
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// 执行项目构建命令：stdout/stderr 重定向到 build.log，跑完返回退出码与耗时。
///
/// - `build_cmd` 为空 → 返回 `AppError::Process`
/// - 构建开始前 truncate `build.log`（保证只含本次输出）
/// - 跑完即退出，不入 registry
pub async fn run_build(project: &Project, logs_root: &Path) -> AppResult<BuildResult> {
    let build_cmd = project.build_cmd.as_ref().ok_or_else(|| {
        AppError::Process(format!("项目「{}」未配置构建命令", project.name))
    })?;
    if build_cmd.trim().is_empty() {
        return Err(AppError::Process(format!(
            "项目「{}」构建命令为空",
            project.name
        )));
    }

    // 构建日志路径 + 构建前 truncate（保证只含本次输出）
    let log_path = crate::logs::paths::log_path_of(
        logs_root,
        &project.name,
        crate::logs::paths::LogType::Build,
    );
    // 确保目录存在（首次构建时目录可能还没建）
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::process::spawn::truncate_log(&log_path);

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
    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::Process(format!("构建「{}」启动失败: {e}", project.name)))?;

    let status = child.wait().await.map_err(|e| {
        AppError::Process(format!("构建「{}」等待退出失败: {e}", project.name))
    })?;

    let exit_code = status.code().unwrap_or(-1);
    let duration_ms = start.elapsed().as_millis() as u64;
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
