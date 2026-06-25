//! 数据库访问层
//!
//! `tauri-plugin-sql` 的 `DbPool::execute/select` 是 `pub(crate)`，应用代码不可用。
//! 本模块从插件的 `DbInstances` managed state 中取出 `DbPool::Sqlite` 变体，
//! 提取内部的 `sqlx::Pool<Sqlite>` 供服务层直查（详见 ADR-005）。

pub mod migrations;

use sqlx::Sqlite;
use tauri::{AppHandle, Manager, Runtime};

use crate::error::{AppError, AppResult};

/// `add_migrations()` 注册时使用的连接 URL，同时作为 `DbInstances` HashMap 的 key。
/// 必须与 `lib.rs` 中 `add_migrations` 第一个参数保持一致。
pub const DB_CONN_URL: &str = "sqlite:devlauncher.db";

/// 从 Tauri managed state 取出 SQLite 连接池的克隆。
pub fn pool<R: Runtime>(app: &AppHandle<R>) -> AppResult<sqlx::Pool<Sqlite>> {
    let instances = app.state::<tauri_plugin_sql::DbInstances>();
    let map = instances.0.blocking_read();

    match map.get(DB_CONN_URL) {
        Some(tauri_plugin_sql::DbPool::Sqlite(p)) => Ok(p.clone()),
        #[allow(unreachable_patterns)]
        Some(_) => Err(AppError::Database(format!(
            "连接 {DB_CONN_URL} 不是 SQLite 类型"
        ))),
        None => Err(AppError::Database(format!(
            "数据库 {DB_CONN_URL} 尚未加载（插件未初始化）"
        ))),
    }
}

/// 把 SQLite UNIQUE 约束错误转为业务错误。
///
/// `on_unique` 在匹配到时调用，可返回具体业务错误（如 `GroupNameExists`）；
/// 未匹配则原样返回 `sqlx::Error`。
pub fn map_unique_err<F>(e: sqlx::Error, on_unique: F) -> AppError
where
    F: FnOnce(String) -> AppError,
{
    if let Some(db_err) = e.as_database_error() {
        if db_err.message().contains("UNIQUE") {
            return on_unique(db_err.message().to_string());
        }
    }
    AppError::Sqlx(e)
}
