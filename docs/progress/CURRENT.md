# CURRENT - 交接状态单

> **每个会话开头读此文件**，了解当前进度与下一步；**每个会话结束前更新此文件**。
> 最后更新：阶段 1 ✅ 完成

## 一、项目概况

- **项目**：DevLauncher — 开发机项目管理器（本地多项目一键启停 / 监控 / 日志）
- **技术栈**：Tauri 2.x + Vue 3 + Element Plus + Pinia + SQLite (tauri-plugin-sql + sqlx 直查)
- **范围**：仅 MVP（详见 `docs/开发计划.md`）

## 二、阶段进度

| 阶段 | 内容 | 状态 |
|---|---|---|
| 0 | 脚手架 + 文档骨架 | ✅ 完成 |
| 1 | DB 层（migration + service CRUD） | ✅ 完成 |
| 2 | 进程托管核心（spawn + Job Object + registry） | ⬜ 下一个 |
| 3 | 项目注册 UI（列表 + 表单 + 分组） | ⬜ 未开始 |
| 4 | 启停 UI + 命令对接 | ⬜ 未开始 |
| 5 | 监控面板（sysinfo + 端口探测） | ⬜ 未开始 |
| 6 | 日志流（实时 tail + 历史分页） | ⬜ 未开始 |
| 7 | 构建命令 + 一键发布 | ⬜ 未开始 |

## 三、当前状态

**阶段 1 已完成**。DB 层（迁移 + 模型 + 服务 + 命令 + 单测）就位：
- ✅ `cargo check` 通过（无 error；warning 均为 state.rs 预留字段）
- ✅ `cargo test`：10 个单测全绿（group 4 个 + project 6 个）
- ✅ `vite build` 成功，dist/ 已重新生成
- ✅ 9 个命令已注册到 invoke_handler（group 4 + project 5）

## 四、下一步 → 阶段 2（进程托管核心）

1. `src-tauri/src/process/mod.rs`：模块入口
2. `spawn.rs`：`tokio::process::Command` + Windows `CREATE_SUSPENDED`/`creation_flags` + stdio 重定向到日志文件
3. `job_object.rs`：Windows Job Object（unsafe Win32 调用），杀掉整个进程树
4. `registry.rs`：`ProcessRegistry`（`HashMap<i64, RunningProcess>`，projectId → job + child + log_path）
5. `tree.rs`：基于 sysinfo 收集后代 PID（用于端口归属校验）
6. 对接命令：`start_project` / `stop_project` / `restart_project`（命令清单见 03-命令清单.md §四）
7. 接入 `AppState`（持有 `ProcessRegistry`）
8. 单测：用简单命令（如 `cmd /C ping`）验证 spawn → stop 回收进程树

## 五、关键决策备忘（详见 docs/adr/）

- **ADR-001 进程管理**：不用 `tauri-plugin-shell`，用 `tokio::process` + Windows Job Object 自管
- **ADR-002 日志推送**：用 `tauri::ipc::Channel`（不用事件系统），前端 GC Channel 自动退订
- **ADR-003 端口探测**：`netstat2` 查监听者 + `TcpStream::connect` 探活双重检测
- **ADR-004 数据库**：用 `tauri-plugin-sql`（SQLite）做迁移注册 + 连接管理
- **ADR-005 DB 服务层访问**：因 `DbPool::execute/select` 是 `pub(crate)`，服务层改从 `DbPool::Sqlite` 取出 `sqlx::Pool<Sqlite>` 直查（修订 ADR-004 实现假设）

## 六、已验证的环境

- Node: v22.22.0
- pnpm: 11.9.0（注意 `ERR_PNPM_IGNORED_BUILDS` 已知问题，见阶段 0 总结）
- Rust: 1.95（stable，x86_64-pc-windows-msvc）
- VS 2022 Professional + C++ NativeDesktop 工作负载（VC Tools 14.39 + Windows SDK）
- Cargo 镜像：rsproxy.cn（配置在 `~/.cargo/config.toml`）
- npm 镜像：npmmirror.com

## 七、阶段 1 已注册命令速查

```
group:    list_groups / create_group / update_group / delete_group
project:  list_projects(group_id?) / get_project / create_project / update_project / delete_project
```

前端 invoke 示例（阶段 3 实现 UI 时用）：
```ts
const groups = await invoke<Group[]>('list_groups')
const projects = await invoke<Project[]>('list_projects', { groupId: 1 })
await invoke('create_project', { input: { name, type: 'node', path, startCmd, ... } })
```

## 八、文件清单

- 阶段 0 产出：详见 `docs/progress/00-scaffold.md`
- 阶段 1 产出：详见 `docs/progress/01-db-layer.md`
