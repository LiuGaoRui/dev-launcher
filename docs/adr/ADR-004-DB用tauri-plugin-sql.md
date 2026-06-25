# ADR-004: 数据库用 tauri-plugin-sql

**状态**：已采纳
**日期**：阶段 0

## 背景

DevLauncher 需要持久化项目配置、分组、日志索引等。SQLite 是嵌入式首选。

Tauri 生态有两个方案：
- **tauri-plugin-sql**：官方插件，前端可直接 `Database.select()`，Rust 端通过 migration 初始化
- **自管 rusqlite + tokio-rusqlite**：纯 Rust 端，异步连接池

## 决策

**使用 tauri-plugin-sql（SQLite backend）。**

## 理由

1. **Migration 机制**：`Migration { version, description, sql }`，`include_str!` 嵌入 SQL 文件，编译期检查存在性。比手写 schema 版本管理简单。

2. **前端直查能力**：简单查询（如 `list_groups`）前端可直接 `Database.execute("SELECT ...")`，无需写 command。MVP 仍走 command 层（统一错误处理），但保留直查灵活性。

3. **统一连接管理**：插件内部管理连接池，无需自己写 `Arc<Mutex<Pool>>`。

4. **类型转换**：自动把 SQL 行转 JSON，前端 `as Group` 即可。

## 代价

- Rust 业务层访问 DB 需通过插件的 `DbPool`（从 State 取），不如 rusqlite 直接
- 无法使用 rusqlite 的高级特性（如 `serde_json::to_row`）
- 全部 SQL 是字符串（无编译期检查），靠 service 层封装缓解

## 实现

```rust
// lib.rs
.plugin(
    tauri_plugin_sql::Builder::default()
        .add_migrations("sqlite:devlauncher.db", migrations())
        .build()
)

// service 层
async fn list_groups(pool: &DbPool) -> AppResult<Vec<Group>> {
    let rows = pool.select("SELECT * FROM `group` ORDER BY `order`", vec![]).await?;
    Ok(rows.into_iter().map(Group::from).collect())
}
```

DB 文件位置：`app_data_dir/devlauncher.db`（由插件自动定位）。

## 相关

- 02-数据库设计.md（DDL）
