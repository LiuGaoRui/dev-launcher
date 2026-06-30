//! 进程树终止：跨平台封装，Windows 用 `taskkill /F /T`。
//!
//! 抽出为公共模块供两处复用：
//! - `commands/process.rs` 的 stop_project（杀托管项目进程树）
//! - `commands/cleaner.rs` 的 kill_dev_processes（批量杀开发进程树）

use crate::error::{AppError, AppResult};

/// 用 `taskkill /F /T /PID <pid>` 杀掉指定进程及其全部后代。
///
/// `/F` 强制终止，`/T` 递归杀子进程树。Windows 自带命令。
#[cfg(windows)]
pub async fn kill_process_tree(pid: u32) -> AppResult<()> {
    use crate::process::spawn::CREATE_NO_WINDOW;

    let output = tokio::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID"])
        .arg(pid.to_string())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .map_err(|e| AppError::Process(format!("调用 taskkill 失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Process(format!(
            "taskkill 终止进程 {pid} 失败: {}",
            stderr.trim()
        )));
    }
    Ok(())
}

/// 非 Windows 平台的占位实现。
///
/// 当前产品仅面向 Windows（taskkill 杀树），其他平台返回错误而非编译失败，
/// 便于在跨平台开发机上 `cargo check`。
#[cfg(not(windows))]
pub async fn kill_process_tree(pid: u32) -> AppResult<()> {
    Err(AppError::Process(format!(
        "当前平台暂不支持终止进程树 (pid={pid})，仅 Windows 可用"
    )))
}
