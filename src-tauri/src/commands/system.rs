//! system 命令薄层：open_url。
//!
//! open_url：用系统默认浏览器打开 URL。仅放行 http/https，避免 `start`
//! 命令注入（Windows 的 start 会解析非 URL 参数为文件/命令）。

use crate::error::{AppError, AppResult};
use crate::process::spawn::CREATE_NO_WINDOW;

/// 用默认浏览器打开 URL。
///
/// 安全：仅接受 http/https 协议，拒绝其他（file://、UNC 路径、可执行文件等），
/// 避免 Windows `start` 命令被注入执行任意程序。
#[tauri::command]
pub async fn open_url(url: String) -> AppResult<()> {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(AppError::Process(format!(
            "仅支持打开 http/https 链接，拒绝: {url}"
        )));
    }

    // Windows：cmd /C start "" <url>，空标题 "" 避免 URL 被当作标题
    // （含 & 等特殊字符的 URL 会被 cmd 截断）。CREATE_NO_WINDOW 避免弹黑框。
    let status = tokio::process::Command::new("cmd")
        .args(["/C", "start", "", trimmed])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .await
        .map_err(|e| AppError::Process(format!("调用默认浏览器失败: {e}")))?;

    if !status.success() {
        return Err(AppError::Process(format!(
            "打开链接失败（exit code: {}）",
            status.code().unwrap_or(-1)
        )));
    }
    Ok(())
}
