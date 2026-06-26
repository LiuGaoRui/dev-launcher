//! scan_root 命令薄层：扫描目录（面板）排序的查询与持久化。
//!
//! 包装 ScanRootService，供前端按拖拽顺序读取/保存扫描目录的排列。

use std::collections::HashMap;

use tauri::{AppHandle, Runtime};

use crate::db;
use crate::error::AppResult;
use crate::services::ScanRootService;

/// 读取全部已记录的扫描目录顺序：scan_root → sort_order。
#[tauri::command]
pub async fn list_scan_root_order<R: Runtime>(
    app: AppHandle<R>,
) -> AppResult<HashMap<String, i64>> {
    let pool = db::pool(&app)?;
    ScanRootService::list_order(&pool).await
}

/// 按给定顺序批量重排扫描目录。roots 的下标即新 sort_order。
#[tauri::command]
pub async fn reorder_scan_roots<R: Runtime>(
    app: AppHandle<R>,
    roots: Vec<String>,
) -> AppResult<()> {
    let pool = db::pool(&app)?;
    ScanRootService::reorder(&pool, roots).await
}
