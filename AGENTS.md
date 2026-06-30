# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

DevLauncher（开发机项目管理器）是一个 **Tauri v2 桌面应用**（Windows），用于统一管理本地开发项目（SpringBoot、Node、Docker Compose 等）的启停、构建、发布，提供实时进程监控和日志查看。

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
├── stores/                   # Pinia stores：project, build, log, theme
├── views/                    # 路由页面：ProjectList, ProjectDetail
├── components/
│   ├── layout/               # AppShell, TitleBar, StatusBar
│   └── project/              # ProjectCard, ActionBar, StatusBadge, MetricsBar, ProjectFormDialog, ProjectScanDialog
├── styles/                   # theme.css（CSS 变量，非 Naive UI 元素的主题）
├── types/                    # TS 类型，对齐 Rust serde 结构体
└── router/index.ts           # hash 路由：/projects, /projects/:id

src-tauri/src/                # Rust 后端
├── commands/                 # IPC handler（薄层，18 个命令）
├── services/                 # 业务逻辑（project_service, detect_service, scan_root_service，含单元测试）
├── process/                  # 进程管理：spawn, registry, tree, monitor, build；job_object 已退役
├── logs/                     # 日志：paths, tail（Channel 实时推送）
├── db/                       # sqlx Pool 提取 + 迁移注册
├── lib.rs                    # Tauri builder 装配 + 命令注册
├── state.rs                  # AppState（ProcessRegistry, sysinfo::System, builds map）
├── error.rs                  # AppError enum（10 variants）+ AppResult<T>
└── models.rs                 # Serde 模型（Project, ProjectInput, DetectedProject, LaunchScheme 等）
```

## IPC 通信模式

1. **请求/响应**：标准 `invoke()` 调用，用于 CRUD 和探活
2. **流式 Channel**：`ipc::Channel<T>` 用于日志 tail 和构建输出推送。前端持有 Channel 引用防止 GC，引用释放后 Rust 端 `send()` 失败自动退出，无需显式 unsubscribe

## 前端状态管理

- `useProjectStore` — 中心 store。管理项目 CRUD + **3 秒轮询** `probe_statuses` 获取运行时状态。轮询由视图组件控制生命周期（mount 启动，unmount 停止）
- `useBuildStore` — 构建状态机：Channel 订阅 → 累积输出 → exit code
- `useLogStore` — 日志查看：live 模式（Channel tail + 自动滚动）/ history 模式（日期选择 + 分页）
- `useThemeStore` — 明暗主题切换，持久化到 localStorage

## 进程生命周期

项目状态机：`Stopped → Starting → Running / Running(Abnormal) → Stopping → Stopped`

- 进程 spawn 使用 `CREATE_BREAKAWAY_FROM_JOB` 避免继承 Tauri Job Object，进程不绑定管理器生命周期
- Job Object（`KILL_ON_JOB_CLOSE`）已退役（`#[allow(dead_code)]`），停止进程改用 `taskkill /F /T /PID`
- 应用重启后通过 `probe_statuses` 重新发现运行中的进程（sysinfo + 端口匹配）
- 托管进程 stdout/stderr 重定向到日志文件；构建进程 pipe 输出通过 Channel 推前端
- 端口检测通过 netstat2 查 TCP 表 + TcpStream::connect 验证

## 数据库

- SQLite，文件位于 `{app_config_dir}/devlauncher.db`
- 4 个迁移文件（`migrations/0001_init.sql` ~ `0004_add_sort_order.sql`），由 tauri-plugin-sql 执行
- 实际查询通过 sqlx 直连（tauri-plugin-sql 的 execute/select 是 `pub(crate)`）
- `expected_ports` 以 JSON TEXT 存储，sqlx `#[sqlx(json)]` 反序列化

## 命名约定与 import 顺序

- **文件命名**：`snake_case.rs`（Rust）、`PascalCase.vue`（组件）、`camelCase.ts`（其他 TS）
- **TS import 顺序**：vue/第三方 → `@/api/...` → `@/stores/...` → `@/types/...` → `@/utils/...` → 相对路径
- **Rust import 顺序**：std → 第三方 → `crate::xxx`
- `@` 别名指向 `src/`（vite.config.ts + tsconfig.json 配置）

## 文档

设计文档在 `docs/` 目录：架构设计、数据库设计、命令清单、目录结构规范、环境搭建。ADR 在 `docs/adr/` 目录（进程管理、日志推送、端口检测、DB 访问等决策记录）。

## 环境要求

- Node.js >= 20（推荐 22），pnpm >= 9
- Rust >= 1.78（stable），MSVC 工具链（Visual Studio Build Tools，"C++ 桌面开发" 工作负载）
