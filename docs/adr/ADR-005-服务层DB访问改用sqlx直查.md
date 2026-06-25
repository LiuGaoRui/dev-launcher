# ADR-005: 服务层 DB 访问改用 sqlx 直查

**状态**：已采纳
**日期**：阶段 1
**修订**：ADR-004（实现假设部分的修订，决策结论不变）

## 背景

ADR-004 决定使用 `tauri-plugin-sql`（SQLite backend），其实现假设提到：

```rust
async fn list_groups(pool: &DbPool) -> AppResult<Vec<Group>> {
    let rows = pool.select("SELECT * FROM `group` ORDER BY `order`", vec![]).await?;
    Ok(rows.into_iter().map(Group::from).collect())
}
```

阶段 1 实现 DB 层时，查阅 `tauri-plugin-sql` v2.4.0 源码发现一个关键事实：

**`DbPool` 上的 `execute()` / `select()` 方法都是 `pub(crate)`，应用代码（本 crate 之外）无法调用。**

```rust
// tauri-plugin-sql/src/wrapper.rs
pub(crate) async fn execute(&self, _query: String, _values: Vec<JsonValue>) -> Result<...>
pub(crate) async fn select(&self, _query: String, _values: Vec<JsonValue>) -> Result<...>
```

只有以下类型是 `pub` 的：
- `DbPool`（枚举本身）：`pub enum DbPool { Sqlite(Pool<Sqlite>), MySql(...), Postgres(...) }`
- `DbInstances`（state 包装）：`pub struct DbInstances(pub RwLock<HashMap<String, DbPool>>)`
- `Builder` / `Migration` / `MigrationKind`：插件装配与迁移注册

因此 ADR-004 假设的 `pool.select()` 服务层写法**不可行**。

## 决策

**服务层直接使用 `sqlx`（从 `DbPool::Sqlite` 变体取出内部 `sqlx::Pool<Sqlite>`）。**

迁移仍由 `tauri-plugin-sql` 注册（保留前端直查能力 + 自动定位 DB 文件到 `app_config_dir`），但 Rust 业务层不再走插件的查询方法，而是：

```rust
// db/mod.rs
pub fn pool(app: &AppHandle) -> AppResult<sqlx::Pool<Sqlite>> {
    let instances = app.state::<DbInstances>();
    let map = instances.0.blocking_read();
    match map.get(DB_CONN_URL) {
        Some(pool) => match pool {
            DbPool::Sqlite(p) => Ok(p.clone()),
            _ => unreachable!(),
        },
        None => Err(AppError::Database("...".into())),
    }
}

// services/group_service.rs
pub async fn list(pool: &sqlx::Pool<Sqlite>) -> AppResult<Vec<Group>> {
    let groups = sqlx::query_as::<_, Group>("SELECT ... FROM `group` ORDER BY `order`")
        .fetch_all(pool)
        .await?;
    Ok(groups)
}
```

`sqlx 0.8.6` 本就是 `tauri-plugin-sql` 的传递依赖，加为直接依赖（仅启用 `sqlite` 子集）不引入新版本，不增加编译体积。

## 理由

1. **ADR-004 结论不变**：仍用 `tauri-plugin-sql` 做迁移注册、连接管理、DB 文件定位。本 ADR 仅修订「Rust 业务层如何查询」这一实现细节。

2. **类型安全**：`sqlx::query_as::<_, Group>` 配合 `#[derive(FromRow)]` 直接得到强类型结构体，比插件返回的 `Vec<IndexMap<String, serde_json::Value>>` 更安全、更高效。

3. **JSON 列透明映射**：`expected_ports`（TEXT 存 JSON）用 sqlx 的 `#[sqlx(json)]` 自动编解码，无需手写 `serde_json::from_str`。

4. **不阻塞前端直查**：前端简单查询仍可用 `@tauri-apps/plugin-sql` 的 `Database.select()`（MVP 走 command 层统一错误处理，但保留直查灵活性）。

5. **单测友好**：服务层接收 `&Pool<Sqlite>`，测试用 `sqlite::memory:` 建池跑迁移，无需 Tauri 运行时。

## 代价

- 增加一个直接依赖（`sqlx`，与传递依赖同版本，零额外体积）
- `DB_CONN_URL` 常量须与 `add_migrations` 的第一个参数保持一致（已用常量集中管理）
- `DbPool` 模式匹配的 `Some(_)` / `_` 分支因只启用 sqlite feature 在编译期不可达（用 `#[allow(unreachable_patterns)]` 标注）

## 相关

- ADR-004（本 ADR 修订其实现假设）
- `src-tauri/src/db/mod.rs`（pool 取出实现）
- `docs/02-数据库设计.md`（DDL）
