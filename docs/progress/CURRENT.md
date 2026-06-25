# CURRENT - 交接状态单

> **每个会话开头读此文件**，了解当前进度与下一步；**每个会话结束前更新此文件**。
> 最后更新：阶段 7 ✅ 完成（MVP 全部功能就位）

## 一、项目概况

- **项目**：DevLauncher — 开发机项目管理器（本地多项目一键启停 / 监控 / 日志 / 构建 / 发布）
- **技术栈**：Tauri 2.x + Vue 3 + Element Plus + Pinia + SQLite (tauri-plugin-sql + sqlx 直查)
- **范围**：MVP 全部完成（详见 `docs/开发计划.md` §八）；后续 P1/P2 为迭代增强

## 二、阶段进度

| 阶段 | 内容 | 状态 |
|---|---|---|
| 0 | 脚手架 + 文档骨架 | ✅ 完成 |
| 1 | DB 层（migration + service CRUD） | ✅ 完成 |
| 2 | 进程托管核心（spawn + Job Object + registry） | ✅ 完成 |
| 3 | 项目注册 UI（列表 + 表单 + 分组） | ✅ 完成 |
| 4 | 启停 UI + 命令对接 | ✅ 完成（阶段 3 已含启停，阶段 5 补真实探测） |
| 5 | 监控面板（sysinfo + 端口探测） | ✅ 完成 |
| 6 | 日志流（实时 tail + 历史分页） | ✅ 完成 |
| 7 | 构建命令 + 一键发布 | ✅ 完成（MVP 完成） |

## 三、当前状态

**阶段 7 已完成，MVP 全部功能就位**。构建与一键发布全通：piped stdout/stderr 实时推 Channel、构建日志对话框、stop→build→start 编排。
- ✅ `cargo check` 通过（6 个既有预留字段 warning，无新增）
- ✅ `cargo test` 32 通过（含阶段 7 新增 3 个 build 单测）
- ✅ `vue-tsc --noEmit`：0 错误
- ✅ `vite build` 成功（ProjectList chunk 15.74kB）

新增能力：
- **后端 `process/build.rs` 模块**：`BuildEvent`（serde tag/content `{kind,data}`）+ `BuildResult` + `run_build`（piped 并发读 → Channel 推送 → 退出码）
- **`build_project` 命令**：执行 build_cmd，stdout/stderr 实时推 Channel，跑完返回退出码/耗时（不入 registry/不用 Job Object）
- **一键发布编排**（前端）：stop → build → start，IPC 边界原则 §4.2 组合在前端
- **构建日志对话框**（BuildDialog.vue）：深色终端样式、stderr 红色着色、四态状态条、关闭即释放订阅
- **ActionBar 加构建/发布按钮**：build_cmd 为空时禁用

## 四、下一步 → P1/P2 迭代（MVP 已完成）

MVP（开发计划 §八）全部就位：项目注册 / 启停 / 重启 / 构建 / 发布 / 监控 / 日志。
后续迭代方向（P1/P2，非阻塞）：
1. **P1 Git 状态**：显示当前分支、未提交数量、未推送数量
2. **P1 IDE 打开**：`idea64.exe <path>` / `code <path>`
3. **P1 Docker Compose**：`docker compose up -d` / `down`
4. **P2 AI 进程识别**：识别 Claude Code / Cursor 并关联项目
5. **P2 项目依赖启动**：启动时检测 Redis/Nacos 等依赖

## 五、构建架构（阶段 7 产出）

```
build_project(id, onEvent: Channel) 入口
   │ 取 project → 校验 build_cmd
   └─ run_build(project, channel)
        ├─ cmd /C <build_cmd> (piped stdout/stderr, CREATE_NO_WINDOW)
        ├─ tokio::spawn pump_pipe(stdout) ─┐
        ├─ tokio::spawn pump_pipe(stderr) ─┤  并发读 4KB 块（防死锁）
        │   on_event.send(Stdout/Stderr) ◀┘  send 失败 → break（Channel GC → kill_on_drop 回收）
        ├─ child.wait().await → exit code
        ├─ on_event.send(Exit(code))
        └─ return BuildResult{exit_code, duration_ms}
```

**与 start/spawn 的区别**：
- start（常驻）：stdout/stderr 追加写日志文件、Job Object 托管、入 registry
- build（一次性）：piped 实时推 Channel、不入 registry、kill_on_drop 兜底、返回退出码

## 六、日志架构（阶段 6 产出）

```
子进程 stdout/stderr → append 落盘 {logs_root}/{name}/{YYYYMMDD}.log
   │
subscribe_log(projectId, onEvent: Channel)
   └─ spawn_tail_task(log_path, channel)
        ├─ push_all_available(offset=0) → 首推今日全部
        └─ loop (250ms): size<offset? 重置重读 / size>offset? 推增量 / send Err? break
        channel.send(LogChunk{offset, text}) ──▶ 前端 onmessage → store.lines 追加
```

## 七、监控架构（阶段 5 产出）

```
前端 ProjectList (onMounted) → startPolling() 每 3s → probe_statuses(ids?)
后端：registry.running_ids() → batch SQL expected_ports → refresh_processes(All) cpu 基线
      → collect_tcp_sockets() netstat2 全表一次 → 逐项目 probe_one（聚合进程树 + 端口归属校验）
返回 Vec<ProjectStatus> → store statuses[id]
```

## 八、关键决策备忘（详见 docs/adr/）

- **ADR-001 进程管理**：不用 `tauri-plugin-shell`，用 `tokio::process` + Windows Job Object 自管
- **ADR-002 日志/构建推送**：用 `tauri::ipc::Channel`（不用事件系统），前端 GC Channel 自动退订 ✅ 阶段 6/7 落地
- **ADR-003 端口探测**：`netstat2` 查监听者 + `TcpStream::connect` 探活双重检测
- **ADR-004 数据库**：用 `tauri-plugin-sql`（SQLite）做迁移注册 + 连接管理
- **ADR-005 DB 服务层访问**：从 `DbPool::Sqlite` 取 `sqlx::Pool<Sqlite>` 直查
- **阶段 3 决策**：目录选择用前端 dialog 插件 `open({directory})` 而非 Rust `pick_directory` 命令
- **阶段 5 决策**：监控指标聚合整树（非仅根进程）；端口做归属校验（防 R3）；固定 3s 轮询（不做启动宽限期）
- **阶段 6 决策**：实时起点=今日全部(offset 0)；固定 250ms 轮询(不引 notify)；截断自动重读；跳过 log_ref 表扫文件系统
- **阶段 7 决策**：构建 piped 实时推 Channel（不落盘）；管道并发读防死锁；一次性进程不入 registry；一键发布编排放前端

## 九、已验证的环境

- Node: v22.22.0
- pnpm: 11.9.0（注意 `ERR_PNPM_IGNORED_BUILDS` 已知问题：直接用 `npx vue-tsc` / `npx vite build` 绕过 deps-status）
- Rust: 1.95（stable，x86_64-pc-windows-msvc）
- VS 2022 Professional + C++ NativeDesktop 工作负载（VC Tools 14.39 + Windows SDK）
- Cargo 镜像：rsproxy.cn
- npm 镜像：npmmirror.com

## 十、已注册命令速查（18 个）

```
group:    list_groups / create_group / update_group / delete_group
project:  list_projects(group_id?) / get_project / create_project / update_project / delete_project
process:  start_project(id) / stop_project(id) / restart_project(id)
          / build_project(id, channel) / probe_statuses(ids?)
log:      subscribe_log(projectId, channel) / read_log_history(projectId, date, offset, limit)
          / list_log_dates(projectId) / clear_log(projectId, date?)
```

## 十一、前端架构（阶段 7 更新）

```
src/
├── types/
│   ├── group.ts / project.ts      # 实体类型 + BuildEvent/BuildResult/StartResult
│   ├── monitor.ts                 # HealthStatus / PortStatus / ProjectStatus + formatBytes
│   └── log.ts                     # LogChunk / LogHistoryPage
├── api/
│   ├── invoke.ts / group.ts / project.ts / monitor.ts / log.ts
│   └── process.ts                 # start/stop/restart/buildProject(Channel)
├── stores/
│   ├── group.ts / project.ts      # project: + statuses Map + 3s 轮询
│   ├── log.ts                     # mode(live/history) + lines + startLive/stopLive/loadHistory
│   └── build.ts                   # ★ lines(stdout/stderr) + running + exitCode + startBuild/stopBuild
├── components/project/
│   ├── StatusBadge.vue / MetricsBar.vue
│   ├── ActionBar.vue              # ★ +构建/发布按钮（canBuild 控制）
│   ├── ProjectCard.vue            # ★ 透传 build/deploy
│   ├── ProjectFormDialog.vue
│   └── BuildDialog.vue            # ★ 构建日志对话框（终端样式 + 四态状态条）
└── views/
    ├── ProjectList.vue            # ★ handleBuild/handleDeploy(stop→build→start) + BuildDialog
    ├── ProjectDetail.vue          # 项目详情（信息卡 + 日志面板实时/历史）
    └── GroupManage.vue            # 分组增删改
```

**IPC 参数约定**（Tauri v2）：
- 顶层命令参数名用 camelCase（Tauri 自动转 snake_case）：`groupId` / `id` / `input` / `ids` / `projectId` / `onEvent` / `date` / `offset` / `limit`
- 嵌套结构体字段名用 snake_case（对齐 Rust serde，不转换）：`group_id` / `start_cmd` / `expected_ports` / `cpu_percent` / `exit_code` / `duration_ms`

## 十二、文件清单

- 阶段 0-3 产出：详见 `docs/progress/00~03-*.md`
- 阶段 5 产出：详见 `docs/progress/05-monitor.md`
- 阶段 6 产出：详见 `docs/progress/06-logs.md`
- 阶段 7 产出：详见 `docs/progress/07-build.md`
