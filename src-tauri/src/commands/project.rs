//! project 命令薄层
//!
//! 签名对齐 docs/03-命令清单.md。从 AppHandle 取 pool，转调 ProjectService。

use tauri::{AppHandle, Runtime};

use crate::db;
use crate::error::AppResult;
use crate::models::{Project, ProjectInput};
use crate::services::ProjectService;

#[tauri::command]
pub async fn list_projects<R: Runtime>(app: AppHandle<R>) -> AppResult<Vec<Project>> {
    let pool = db::pool(&app)?;
    ProjectService::list(&pool).await
}

#[tauri::command]
pub async fn get_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<Project> {
    let pool = db::pool(&app)?;
    ProjectService::get(&pool, id).await
}

#[tauri::command]
pub async fn create_project<R: Runtime>(
    app: AppHandle<R>,
    input: ProjectInput,
) -> AppResult<Project> {
    let pool = db::pool(&app)?;
    ProjectService::create(&pool, input).await
}

#[tauri::command]
pub async fn update_project<R: Runtime>(
    app: AppHandle<R>,
    id: i64,
    input: ProjectInput,
) -> AppResult<Project> {
    let pool = db::pool(&app)?;
    ProjectService::update(&pool, id, input).await
}

#[tauri::command]
pub async fn delete_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<()> {
    let pool = db::pool(&app)?;
    ProjectService::delete(&pool, id).await
}

/// 按 ids 顺序批量重排项目 sort_order（前端拖拽排序后调用）。
#[tauri::command]
pub async fn reorder_projects<R: Runtime>(
    app: AppHandle<R>,
    ids: Vec<i64>,
) -> AppResult<()> {
    let pool = db::pool(&app)?;
    ProjectService::reorder(&pool, ids).await
}
