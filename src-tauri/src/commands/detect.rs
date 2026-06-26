//! detect 命令薄层
//!
//! 包装 DetectService::scan，供前端「扫描根目录」调用。

use crate::error::AppResult;
use crate::models::DetectedProject;
use crate::services::DetectService;

/// 扫描根目录下所有可识别的 Java / Node 项目。
///
/// - 入参 `root`：绝对路径
/// - 出参：检测到的项目列表（含推断的启动方案、端口）
#[tauri::command]
pub async fn scan_projects(root: String) -> AppResult<Vec<DetectedProject>> {
    DetectService::scan(&root).await
}
