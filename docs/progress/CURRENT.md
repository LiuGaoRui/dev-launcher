# CURRENT - 交接状态单

> **每个会话开头读此文件**，了解当前进度与下一步；**每个会话结束前更新此文件**。
> 最后更新：阶段 2 ✅ 完成

## 一、项目概况

- **项目**：DevLauncher — 开发机项目管理器（本地多项目一键启停 / 监控 / 日志）
- **技术栈**：Tauri 2.x + Vue 3 + Element Plus + Pinia + SQLite (tauri-plugin-sql + sqlx 直查)
- **范围**：仅 MVP（详见 `docs/开发计划.md`）

## 二、阶段进度

| 阶段 | 内容 | 状态 |
|---|---|---|
| 0 | 脚手架 + 文档骨架 | ✅ 完成 |
| 1 | DB 层（migration + service CRUD） | ✅ 完成 |
| 2 | 进程托管核心（spawn + Job Object + registry） | ✅ 完成 |
| 3 | 项目注册 UI（列表 + 表单 + 分组） | ⬜ 下一个 |
| 4 | 启停 UI + 命令对接 | ⬜ 未开始 |
| 5 | 监控面板（sysinfo + 端口探测） | ⬜ 未开始 |
| 6 | 日志流（实时 tail + 历史分页） | ⬜ 未开始 |
| 7 | 构建命令 + 一键发布 | ⬜ 未开始 |

## 三、当前状态

**阶段 2 已完成**。进程托管核心就位：
- ✅ `cargo check` 通过（仅 state.rs 预留字段 warning）
- ✅ `cargo test`：18 单测全绿（阶段1 的 10 + 阶段2 的 8），2 集成测试 ignored
- ✅ `cargo test -- --ignored`：真实 spawn/stop 集成测试通过（Job Object 杀整树验证）
- ✅ `vite build` 成功
- ✅ 12 个命令已注册（group 4 + project 5 + process 3）

## 四、下一步 → 阶段 3（项目注册 UI）

前端实现，对接已有 9 个 DB 命令 + 3 个进程命令：
1. `src/types/{group,project}.ts` — TS 类型对齐 Rust serde
2. `src/api/{invoke,group,project,process}.ts` — Tauri invoke 薄包装 + 统一错误解析
3. `src/stores/{project,group}.ts` — Pinia
4. `src/views/ProjectList.vue` — 分组树 + 项目卡片
5. `src/components/project/{ProjectCard,StatusBadge,ActionBar}.vue`
6. `src/views/GroupManage.vue` + ProjectForm（dialog 内嵌）
7. `pick_directory` 命令（commands/project.rs，接 dialog 插件）

## 五、关键决策备忘（详见 docs/adr/）

- **ADR-001 进程管理**：不用 `tauri-plugin-shell`，用 `tokio::process` + Windows Job Object 自管；CREATE_SUSPENDED 不可行，改 spawn-then-assign（见附录）
- **ADR-002 日志推送**：用 `tauri::ipc::Channel`（不用事件系统），前端 GC Channel 自动退订
- **ADR-003 端口探测**：`netstat2` 查监听者 + `TcpStream::connect` 探活双重检测
- **ADR-004 数据库**：用 `tauri-plugin-sql`（SQLite）做迁移注册 + 连接管理
- **ADR-005 DB 服务层访问**：因 `DbPool::execute/select` 是 `pub(crate)`，服务层改从 `DbPool::Sqlite` 取出 `sqlx::Pool<Sqlite>` 直查

## 六、已验证的环境

- Node: v22.22.0
- pnpm: 11.9.0（注意 `ERR_PNPM_IGNORED_BUILDS` 已知问题，见阶段 0 总结）
- Rust: 1.95（stable，x86_64-pc-windows-msvc）
- VS 2022 Professional + C++ NativeDesktop 工作负载（VC Tools 14.39 + Windows SDK）
- Cargo 镜像：rsproxy.cn
- npm 镜像：npmmirror.com

## 七、已注册命令速查

```
group:    list_groups / create_group / update_group / delete_group
project:  list_projects(group_id?) / get_project / create_project / update_project / delete_project
process:  start_project(id) / stop_project(id) / restart_project(id)
```

阶段 3 前端 invoke 示例：
```ts
const projects = await invoke<Project[]>('list_projects', { groupId: 1 })
const result = await invoke<StartResult>('start_project', { id: 1 })
await invoke('stop_project', { id: 1 })
```

## 八、文件清单

- 阶段 0 产出：详见 `docs/progress/00-scaffold.md`
- 阶段 1 产出：详见 `docs/progress/01-db-layer.md`
- 阶段 2 产出：详见 `docs/progress/02-process-core.md`
