//! 业务服务层
//!
//! 纯业务逻辑，不依赖 Tauri，接收 `&Pool<Sqlite>`。便于单测。

pub mod detect_service;
pub mod group_service;
pub mod project_service;

pub use detect_service::DetectService;
pub use group_service::GroupService;
pub use project_service::ProjectService;

/// 测试共享：创建 in-memory SQLite pool 并执行迁移。
#[cfg(test)]
pub(crate) async fn setup_pool() -> sqlx::Pool<sqlx::Sqlite> {
    use crate::db::migrations;
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    for stmt in migrations::migrations()[0].sql.split(';') {
        let t = stmt.trim();
        if t.is_empty() {
            continue;
        }
        sqlx::query(t).execute(&pool).await.expect("migrate");
    }
    pool
}
