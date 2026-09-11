# AGENTS.md

本文件为在本仓库中工作的 AI 编码代理提供架构与约定指引（`CLAUDE.md` 通过 `@AGENTS.md` 引用本文件）。

## 项目概述

DevLauncher（开发机项目管理器）是一个 **Tauri v2 桌面应用**（Windows），用于统一管理本地开发项目（SpringBoot、Node、Docker Compose 等）的启停、构建、发布，提供实时进程监控、日志查看、内存清理与端口监控。

## 常用命令

```bash
# 开发（启动 Vite + Tauri 桌面窗口）
pnpm tauri dev

# 仅前端开发（浏览器，端口 1420）
pnpm dev

# TypeScript 类型检查
pnpm typecheck          # vue-tsc --noEmit

# 构建生产包（MSI + NSIS installer）
pnpm tauri build

# Rust 后端检查
cargo check             # 在 src-tauri/ 目录下

# Rust 测试
cargo test              # 在 src-tauri/ 目录下
```

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3 (Composition API, `<script setup>`), Naive UI 2.x, Pinia 2.x, Vue Router 4.x, Vite 5.x, TypeScript 5.6 |
| 后端 | Rust edition 2021, Tauri 2.x, Tokio, sqlx 0.8 (SQLite) |
| 进程管理 | sysinfo 0.32, netstat2 0.9, Windows Job Objects（已退役，保留代码备用） |
| 包管理 | pnpm 9+, Cargo |

## 架构：三层 + IPC 边界

```
前端 (Vue 3 + Naive UI + Pinia)
  │  Tauri IPC (invoke / Channel)
  ▼
Rust commands/  ──薄层──▶  services/  ──▶  process/ / logs/
  │                              │
  │  tauri-plugin-sql            │  sqlx 直连
  ▼                              ▼
         SQLite (devlauncher.db)
```

**核心原则**：
- `commands/` 只做参数解包 + 调用 service + 错误转换，不含业务逻辑
- `services/` 无 Tauri 依赖，接收 `&Pool<Sqlite>`，可独立单测
- 多步工作流（如"一键部署"= stop + build + start）由前端编排，Rust 只提供原子操作
- 进程不绑定管理器生命周期 —— 应用重启后通过 `probe_statuses` 重新发现运行中的进程

## 关键目录

```
src/                          # Vue 3 前端
├── api/                      # Tauri invoke 薄包装，一个文件一个域
│   └── invoke.ts             # invokeCmd<T>(), safeCall() 返回 [data, error]
├── stores/                   # Pinia stores：project, build, log, cleaner, port, theme
├── views/                    # 路由页面：ProjectList, ProjectDetail
├── components/
│   ├── layout/               # AppShell, TitleBar, StatusBar
│   ├── project/              # ProjectCard, ActionBar, StatusBadge, MetricsBar, ProjectFormDialog, ProjectScanDialog
│   ├── cleaner/              # CleanerDrawer（内存清理器）
│   └── port/                 # PortMonitorModal（端口监控）
├── styles/                   # theme.css（CSS 变量，非 Naive UI 元素的主题）
├── types/                    # TS 类型，对齐 Rust serde 结构体
└── router/index.ts           # hash 路由：/（重定向）, /projects, /projects/:id

src-tauri/src/                # Rust 后端
├── commands/                 # IPC handler（薄层，31 个命令，8 个文件）
├── services/                 # 业务逻辑（project_service, detect_service, scan_root_service, cleaner_lock_service, port_ignore_service，含单元测试）
├── process/                  # 进程管理：spawn, registry, tree, monitor, build, kill, dev_scan, port_scan, trim；job_object 已退役
├── logs/                     # 日志：paths（按项目 id 分目录）, tail（轮询增量 + Channel 推送）, subscription（订阅注册表）
├── db/                       # sqlx Pool 提取 + 迁移注册
├── lib.rs                    # Tauri builder 装配 + 命令注册
├── state.rs                  # AppState（ProcessRegistry, builds map, log_subs, start_locks, sysinfo::System）
├── error.rs                  # AppError enum（10 variants）+ AppResult<T>
└── models.rs                 # Serde 模型（Project, ProjectInput, DetectedProject, LaunchScheme 等）
```

## IPC 通信模式

1. **请求/响应**：标准 `invoke()` 调用，用于 CRUD 和探活
2. **流式 Channel（仅日志实时 tail）**：`subscribe_log` 登记订阅并返回后端分配的 `sub_id`，前端切换订阅或离开页面时显式调 `unsubscribe_log(sub_id)` 取消（ADR-006，取代 ADR-002 的「前端 GC 自动退订」）。`LogChunk` 携带 `sub_id`，前端据此丢弃过期订阅的推送；Channel 关闭导致 `send()` 失败时 tail task 仍会退出，作为兜底
3. **构建不走 Channel**：构建输出写入 `{logs_root}/{project_id}/build.log`，前端每 1 秒轮询 `get_build_status` 取运行状态与退出码，输出经日志订阅查看

## 前端状态管理

- `useProjectStore` — 中心 store。管理项目 CRUD + **3 秒轮询** `probe_statuses` 获取运行时状态。轮询由视图组件控制生命周期（mount 启动，unmount 停止）
- `useBuildStore` — 构建状态：`build_project` 立即返回后每 **1 秒轮询** `get_build_status`（递归 `setTimeout`，天然串行）直到 `running === false`；不累积输出文本
- `useLogStore` — 日志查看：仅实时模式（Channel tail + 自动滚动）。每次订阅递增 token 并校验 `chunk.sub_id`，过期推送一律丢弃；`stopLive` 显式调 `unsubscribe_log`，不依赖 GC
- `useCleanerStore` / `usePortStore` — 内存清理器 / 端口监控
- `useThemeStore` — 明暗主题切换，持久化到 localStorage

## 进程生命周期

项目状态机：`Stopped → Starting → Running / Running(Abnormal) → Stopping → Stopped`

- 进程 spawn 使用 `CREATE_BREAKAWAY_FROM_JOB` 避免继承 Tauri Job Object，进程不绑定管理器生命周期
- Job Object（`KILL_ON_JOB_CLOSE`）已退役（`#[allow(dead_code)]`），停止进程改用 `taskkill /F /T /PID`
- 应用重启后通过 `probe_statuses` 重新发现运行中的进程（sysinfo + 端口匹配）
- 托管进程与构建进程的 stdout/stderr 均重定向到日志文件（`{logs_root}/{project_id}/{start|build}.log`），spawn/构建前 truncate，只保留本次输出，由 tail task 实时订阅推送
- 同一项目的「校验 → truncate → spawn → 入 registry」由 per-project `tokio::sync::Mutex` 串行化（`AppState::start_lock`），避免并发启动重复 truncate 日志
- 端口检测通过 netstat2 查 TCP 表 + TcpStream::connect 验证

## 数据库

- SQLite，文件位于 `{app_config_dir}/devlauncher.db`
- 6 个迁移文件（`src-tauri/migrations/0001_init.sql` ~ `0006_add_port_ignore.sql`），由 tauri-plugin-sql 执行
- 实际查询通过 sqlx 直连（tauri-plugin-sql 的 execute/select 是 `pub(crate)`）
- `expected_ports` 以 JSON TEXT 存储，sqlx `#[sqlx(json)]` 反序列化

## 命名约定与 import 顺序

- **文件命名**：`snake_case.rs`（Rust）、`PascalCase.vue`（组件）、`camelCase.ts`（其他 TS）
- **TS import 顺序**：vue/第三方 → `@/api/...` → `@/stores/...` → `@/types/...` → `@/utils/...` → 相对路径
- **Rust import 顺序**：std → 第三方 → `crate::xxx`
- `@` 别名指向 `src/`（vite.config.ts + tsconfig.json 配置）

## 文档

设计文档在 `docs/` 目录：架构设计、数据库设计、命令清单、目录结构规范、环境搭建。ADR 在 `docs/adr/` 目录，共 6 篇：ADR-001 进程管理、ADR-002 日志/构建推送用 Channel（其「GC 自动退订」部分已被 ADR-006 取代）、ADR-003 端口探测、ADR-004 用 tauri-plugin-sql、ADR-005 服务层改用 sqlx 直查、ADR-006 日志订阅生命周期与项目隔离。

> 注意：`docs/03-命令清单.md` 是设计期文档，仍写「18 个命令」且含已删除的命令（如 `restart_project`、`read_log_history`、`list_groups`）。命令签名以 `src-tauri/src/lib.rs` 的注册表为准。

## 环境要求

- Node.js >= 20（推荐 22），pnpm >= 9
- Rust >= 1.78（stable），MSVC 工具链（Visual Studio Build Tools，"C++ 桌面开发" 工作负载）
