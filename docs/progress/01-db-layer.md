# 阶段 1 - DB 层完成总结

> 完成：迁移 SQL + models + services + commands + 10 个单测全绿

## 产出物

### 迁移
- `src-tauri/migrations/0001_init.sql` — 4 表 DDL（group / project / log_ref / dependency）+ 索引，照搬 02-数据库设计.md

### Rust 后端新增模块
- `src-tauri/src/db/mod.rs` — `pool(app)` 从 `DbInstances` 取出 `sqlx::Pool<Sqlite>`；`DB_CONN_URL` 常量
- `src-tauri/src/db/migrations.rs` — `migrations()` 注册迁移
- `src-tauri/src/models.rs` — `Group` / `GroupInput` / `GroupUpdate` / `Project` / `ProjectType` / `ProjectInput`
- `src-tauri/src/services/mod.rs` — 服务层入口
- `src-tauri/src/services/group_service.rs` — GroupService：list/get/create/update/delete
- `src-tauri/src/services/project_service.rs` — ProjectService：list(可选 group_id)/get/create/update/delete
- `src-tauri/src/commands/mod.rs` — 命令薄层入口
- `src-tauri/src/commands/group.rs` — 4 个 group 命令
- `src-tauri/src/commands/project.rs` — 5 个 project 命令

### Rust 后端改动
- `src-tauri/src/lib.rs` — 声明 5 个新模块；`migrations()` 返回 `db::migrations::migrations()`；`invoke_handler` 注册 9 个命令
- `src-tauri/src/error.rs` — 新增 `AppError::Sqlx(#[from] sqlx::Error)`
- `src-tauri/Cargo.toml` — 加 `sqlx 0.8` 直接依赖（sqlite 子集）

### 文档
- `docs/adr/ADR-005-服务层DB访问改用sqlx直查.md` — 修订 ADR-004 实现假设

## 已注册的 9 个命令

| 模块 | 命令 | 签名 |
|---|---|---|
| group | `list_groups` | `(app) -> Vec<Group>` |
| group | `create_group` | `(app, input: GroupInput) -> Group` |
| group | `update_group` | `(app, id, input: GroupUpdate) -> Group` |
| group | `delete_group` | `(app, id) -> ()` |
| project | `list_projects` | `(app, group_id: Option<i64>) -> Vec<Project>` |
| project | `get_project` | `(app, id) -> Project` |
| project | `create_project` | `(app, input: ProjectInput) -> Project` |
| project | `update_project` | `(app, id, input: ProjectInput) -> Project` |
| project | `delete_project` | `(app, id) -> ()` |

## 验证结果

- ✅ `cargo check` 通过（无 error；warning 均为 state.rs 的预留字段，与阶段 1 无关）
- ✅ `cargo test`：10 个单测全绿
  - group_service：create_get_list / create_duplicate_name_errors / update_fields / delete_sets_project_group_null
  - project_service：create_get / list_filter_by_group / update_changes_fields / create_with_invalid_group_errors / delete_removes_project / ports_json_roundtrip_empty
- ✅ `vite build` 成功，dist/ 已重新生成（generate_context! 不 panic）

## 关键技术决策

### tauri-plugin-sql 的 pub(crate) 限制
ADR-004 假设服务层用 `pool.select()/execute()`，但这两个方法是 `pub(crate)`，应用代码不可用。
解决：从 `DbPool::Sqlite` 变体取出内部 `sqlx::Pool<Sqlite>`，服务层直接用 sqlx。
详见 ADR-005。

### sqlx enum rename
`ProjectType` 枚举需同时配 `#[serde(rename_all = "snake_case")]`（前端 JSON）与
`#[sqlx(type_name = "TEXT", rename_all = "snake_case")]`（DB TEXT 列编解码），
否则 sqlx 解码时报 `invalid value "node" for enum ProjectType`。

### SQLite 内存 DB 测试
测试用 `sqlx::SqlitePool::connect("sqlite::memory:")` 建池，按 `;` 分割迁移 SQL 逐条执行。
（避免 `SqliteConnectOptions::from_url` 的 `tauri::Url` 类型冲突。）

## 遇到的问题与解决

### 1. `From<sqlx::Error>` 未实现
- **现象**：服务层 `?` 报 `From<sqlx::Error> is not implemented for AppError`
- **解决**：error.rs 新增 `AppError::Sqlx(#[from] sqlx::Error)`

### 2. `sqlx::query_as` 要求 `&str`
- **现象**：`project_select_sql` 返回 `String`，传给 `query_as` 类型不匹配
- **解决**：调用处先 `let sql = ...;` 再传 `&sql`

### 3. sqlx enum 解码失败
- **现象**：`invalid value "node" for enum ProjectType`
- **解决**：`ProjectType` 加 `#[sqlx(rename_all = "snake_case")]`

### 4. `SqliteConnectOptions::from_url` 类型冲突
- **现象**：`expected &Url, found &str`（tauri 重导出的 `Url` 与 sqlx 的冲突）
- **解决**：测试改用 `SqlitePool::connect("sqlite::memory:")`（接受 &str）

## 下一步 → 阶段 2（进程托管核心）

1. `src-tauri/src/process/` 模块
2. `spawn.rs`：tokio::process + creation_flags + stdio→文件
3. `job_object.rs`：Windows Job Object（unsafe）
4. `registry.rs`：ProcessRegistry（projectId → job + child）
5. `tree.rs`：基于 sysinfo 收集后代 PID
6. 对接 start_project / stop_project / restart_project 命令
