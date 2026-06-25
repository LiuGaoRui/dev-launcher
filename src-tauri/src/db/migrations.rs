//! 数据库迁移注册
//!
//! 通过 `tauri-plugin-sql` 的 Migration 机制注册，`include_str!` 在编译期
//! 将 SQL 文件嵌入二进制，分发无需带迁移文件。
//! 迁移由插件在连接 DB 时自动执行；DB 文件定位由插件决定（`app_config_dir`）。

use tauri_plugin_sql::{Migration, MigrationKind};

/// 返回全部迁移，按 version 升序。lib.rs 注册插件时调用。
pub fn migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "init",
        sql: include_str!("../../migrations/0001_init.sql"),
        kind: MigrationKind::Up,
    }]
}
