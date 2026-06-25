# CURRENT - 交接状态单

> **每个会话开头读此文件**，了解当前进度与下一步；**每个会话结束前更新此文件**。
> 最后更新：阶段 5 ✅ 完成

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
| 3 | 项目注册 UI（列表 + 表单 + 分组） | ✅ 完成 |
| 4 | 启停 UI + 命令对接 | ✅ 完成（阶段 3 已含启停，阶段 5 补真实探测） |
| 5 | 监控面板（sysinfo + 端口探测） | ✅ 完成 |
| 6 | 日志流（实时 tail + 历史分页） | ⬜ 下一个 |
| 7 | 构建命令 + 一键发布 | ⬜ 未开始 |

## 三、当前状态

**阶段 5 已完成**。监控面板全就位：真实探测替换本地 runningIds、CPU/内存/端口实时展示、三态状态机。
- ✅ `cargo check` 通过（仅 4 个预留字段 warning：data_dir/log_path 留待阶段 6 日志、HealthStatus::Stopped 留待详情页构造）
- ✅ `cargo test` 21 通过（含阶段 5 新增 3 个 monitor 单测）
- ✅ `vue-tsc --noEmit`：0 错误
- ✅ `vite build` 成功

新增能力：
- **后端 `probe_statuses(ids?)` 命令**：批量探测运行中项目，聚合进程树 CPU/内存 + 端口归属校验
- **前端 3s 轮询**：ProjectList 挂载时 startPolling、卸载时 stopPolling，store statuses Map
- **卡片实时指标**：MetricsBar 显示 CPU%/内存/端口监听状态（监听中/被占用/未监听）
- **三态状态徽标**：running(绿脉动) / running_abnormal(橙+警告图标) / stopped(灰)
- **端口归属校验**（风险 R3）：端口 Listen 时校验占用者 PID 是否属于本项目进程树，杜绝误判

## 四、下一步 → 阶段 6（日志流）

1. 后端：spawn 时已重定向 stdout/stderr 到 `{logs_root}/{name}/{YYYYMMDD}.log`（阶段 2 就绪）
2. 加命令：
   - `tail_logs(projectId, channelId)`：用 `tauri::ipc::Channel` 推送实时增量（ADR-002）
   - `read_logs(projectId, date?, offset?, limit?)`：历史分页读取
3. 前端：日志面板（项目详情页内），订阅 Channel 实时流 + 历史分页切换
4. ProjectDetail.vue 实现（目前是 PlaceholderPage）

## 五、监控架构（阶段 5 产出）

```
前端 ProjectList (onMounted)
   │ startPolling()  ─── 每 3s ───┐
   │                               ▼
   │   probe_statuses(ids?)  ← IPC invoke
   │                               │
后端 commands/process.rs           │
   ├─ registry.running_ids()       │
   ├─ batch SQL 取 expected_ports  │
   ├─ system.lock() refresh_processes(All)  ← cpu 基线（持久化 System）
   ├─ collect_tcp_sockets()  ← netstat2 一次查全表，复用
   └─ 逐项目 probe_one():
        ├─ collect_tree(pid) → HashSet<u32>  ← sysinfo 后代收集
        ├─ aggregate_tree → CPU%/内存 bytes 求和
        ├─ check_port(port) → (listening, owned)  ← 端口归属校验
        └─ health 判定: stopped / running / running_abnormal
   │                               │
   └───────────────────────────────┘
        返回 Vec<ProjectStatus> → store statuses[id]
```

**关键设计**：
- 指标聚合整树（npm→node、mvn→java 子进程算进来，避免显示 0）
- 端口归属校验：占用者 PID 不在进程树内 → `owned=false` → 即使 listening 也标 abnormal
- CPU 准确性：sysinfo `cpu_usage()` 需两次 refresh 间隔，AppState 持久化 System，3s 轮询天然形成基线
- 固定 3s 轮询：不做启动宽限期，慢启动项目端口就绪前显示 abnormal，就绪后自动恢复

## 六、关键决策备忘（详见 docs/adr/）

- **ADR-001 进程管理**：不用 `tauri-plugin-shell`，用 `tokio::process` + Windows Job Object 自管
- **ADR-002 日志推送**：用 `tauri::ipc::Channel`（不用事件系统），前端 GC Channel 自动退订
- **ADR-003 端口探测**：`netstat2` 查监听者 + `TcpStream::connect` 探活双重检测
- **ADR-004 数据库**：用 `tauri-plugin-sql`（SQLite）做迁移注册 + 连接管理
- **ADR-005 DB 服务层访问**：从 `DbPool::Sqlite` 取 `sqlx::Pool<Sqlite>` 直查
- **阶段 3 决策**：目录选择用前端 dialog 插件 `open({directory})` 而非 Rust `pick_directory` 命令
- **阶段 5 决策**：监控指标聚合整树（非仅根进程）；端口做归属校验（防 R3）；固定 3s 轮询（不做启动宽限期）

## 七、已验证的环境

- Node: v22.22.0
- pnpm: 11.9.0（注意 `ERR_PNPM_IGNORED_BUILDS` 已知问题：直接用 `npx vue-tsc` / `npx vite build` 绕过 deps-status）
- Rust: 1.95（stable，x86_64-pc-windows-msvc）
- VS 2022 Professional + C++ NativeDesktop 工作负载（VC Tools 14.39 + Windows SDK）
- Cargo 镜像：rsproxy.cn
- npm 镜像：npmmirror.com

## 八、已注册命令速查（13 个）

```
group:    list_groups / create_group / update_group / delete_group
project:  list_projects(group_id?) / get_project / create_project / update_project / delete_project
process:  start_project(id) / stop_project(id) / restart_project(id) / probe_statuses(ids?)
```

阶段 6 拟新增：`tail_logs(projectId, channel)` / `read_logs(projectId, date?, offset?, limit?)`

## 九、前端架构（阶段 5 更新）

```
src/
├── types/
│   ├── group.ts / project.ts      # 实体类型（对齐 Rust serde snake_case）
│   └── monitor.ts                 # ★ HealthStatus / PortStatus / ProjectStatus + formatBytes
├── api/
│   ├── invoke.ts / group.ts / project.ts / process.ts
│   └── monitor.ts                 # ★ probeStatuses(ids?)
├── stores/
│   ├── group.ts                   # groups[] CRUD
│   └── project.ts                 # ★ projects[] + statuses Map + 3s 轮询(startPolling/stopPolling/probeNow)
├── components/project/
│   ├── StatusBadge.vue            # ★ 三态（running/running_abnormal/stopped）
│   ├── MetricsBar.vue             # ★ CPU/内存/端口实时指标
│   ├── ActionBar.vue              # 启动/停止/重启/编辑/删除
│   ├── ProjectCard.vue            # ★ 接 status prop，运行时显示 MetricsBar
│   └── ProjectFormDialog.vue      # 新建/编辑 Dialog
└── views/
    ├── ProjectList.vue            # ★ startPolling/stopPolling 生命周期
    └── GroupManage.vue            # 分组增删改
```

**IPC 参数约定**（Tauri v2）：
- 顶层命令参数名用 camelCase（Tauri 自动转 snake_case）：`groupId` / `id` / `input` / `ids`
- 嵌套结构体字段名用 snake_case（对齐 Rust serde，不转换）：`group_id` / `start_cmd` / `expected_ports` / `cpu_percent`

## 十、文件清单

- 阶段 0-3 产出：详见 `docs/progress/00~03-*.md`
- 阶段 5 产出：详见 `docs/progress/05-monitor.md`
