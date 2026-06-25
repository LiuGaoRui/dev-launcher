//! group 命令薄层
//!
//! 签名对齐 docs/03-命令清单.md。从 AppHandle 取 pool，转调 GroupService。

use tauri::{AppHandle, Runtime};

use crate::db;
use crate::error::AppResult;
use crate::models::{Group, GroupInput, GroupUpdate};
use crate::services::GroupService;

#[tauri::command]
pub async fn list_groups<R: Runtime>(app: AppHandle<R>) -> AppResult<Vec<Group>> {
    let pool = db::pool(&app)?;
    GroupService::list(&pool).await
}

#[tauri::command]
pub async fn create_group<R: Runtime>(
    app: AppHandle<R>,
    input: GroupInput,
) -> AppResult<Group> {
    let pool = db::pool(&app)?;
    GroupService::create(&pool, input).await
}

#[tauri::command]
pub async fn update_group<R: Runtime>(
    app: AppHandle<R>,
    id: i64,
    input: GroupUpdate,
) -> AppResult<Group> {
    let pool = db::pool(&app)?;
    GroupService::update(&pool, id, input).await
}

#[tauri::command]
pub async fn delete_group<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<()> {
    let pool = db::pool(&app)?;
    GroupService::delete(&pool, id).await
}
