# CURRENT - 交接状态单

> **每个会话开头读此文件**，了解当前进度与下一步；**每个会话结束前更新此文件**。
> 最后更新：阶段 3 ✅ 完成

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
| 4 | 启停 UI + 命令对接 | 🟡 部分（启停已通，监控/日志留待 5/6） |
| 5 | 监控面板（sysinfo + 端口探测） | ⬜ 下一个 |
| 6 | 日志流（实时 tail + 历史分页） | ⬜ 未开始 |
| 7 | 构建命令 + 一键发布 | ⬜ 未开始 |

## 三、当前状态

**阶段 3 已完成**。项目注册 + 启停 + 分组 UI 全就位：
- ✅ `vue-tsc --noEmit`：0 错误
- ✅ `vite build` 成功（产物含 ProjectList / GroupManage 分块）
- ✅ `cargo check` 通过（仅 state.rs 2 个预留字段 warning，与阶段 2 一致）

UI 能力（均已对接 12 个后端命令）：
- 项目列表：分组 tab 筛选（全部/未分组/各分组）+ 项目卡片网格
- 项目卡片：名称/类型/路径/启动命令/端口/PID + 运行态徽标
- 项目 CRUD：新建/编辑（Dialog + 表单校验）、删除（确认 + 运行中禁删）
- 启停重启：启动/停止/重启，按钮 busy 跟踪，错误 ElMessage 提示
- 分组管理：新建、行内重命名、上移/下移改 order、删除
- 目录选择：前端直连 `@tauri-apps/plugin-dialog` 的 `open({directory:true})`

## 四、运行态跟踪说明（阶段 3 → 5 的过渡）

阶段 3 尚无「批量查询运行中项目」的后端命令（ProcessRegistry.running_ids 已就绪但未暴露命令）。
前端用本地 `Set<number>` 记忆「本 session 内被前端启动且未停止」的 project_id。

**已知限制**：
- 重启应用后，registry 必空，所有项目 UI 显示「已停止」——符合实际（进程被 Job Object 回收）
- 若项目在应用外被手动 kill，前端状态不会即时更新
- **阶段 5** 会加 `is_running` / `get_status` 批量查询命令 + 端口探测，替换此简化逻辑

## 五、下一步 → 阶段 4/5（监控面板）

优先阶段 5（监控），因为阶段 3 已把启停 UI 做了：
1. 后端加命令：`list_running`（返 running_ids + snapshot）+ `probe_status(id)`（sysinfo CPU/内存 + 端口探测）
2. 前端 store 用真实探测替换 `runningIds` 本地 Set
3. 监控面板：定时（3s）拉取运行态，卡片显示 CPU/内存/端口监听
4. 状态机：Running / Running(Abnormal)（进程在但端口未监听）/ Stopped

## 六、关键决策备忘（详见 docs/adr/）

- **ADR-001 进程管理**：不用 `tauri-plugin-shell`，用 `tokio::process` + Windows Job Object 自管
- **ADR-002 日志推送**：用 `tauri::ipc::Channel`（不用事件系统），前端 GC Channel 自动退订
- **ADR-003 端口探测**：`netstat2` 查监听者 + `TcpStream::connect` 探活双重检测
- **ADR-004 数据库**：用 `tauri-plugin-sql`（SQLite）做迁移注册 + 连接管理
- **ADR-005 DB 服务层访问**：从 `DbPool::Sqlite` 取 `sqlx::Pool<Sqlite>` 直查
- **阶段 3 新增**：目录选择用前端 dialog 插件 `open({directory})` 而非 Rust `pick_directory` 命令
  ——`dialog:default` 权限已含 `allow-open`，无需新增后端代码，更简单

## 七、已验证的环境

- Node: v22.22.0
- pnpm: 11.9.0（注意 `ERR_PNPM_IGNORED_BUILDS` 已知问题：直接用 `npx vue-tsc` / `npx vite build` 绕过 deps-status）
- Rust: 1.95（stable，x86_64-pc-windows-msvc）
- VS 2022 Professional + C++ NativeDesktop 工作负载（VC Tools 14.39 + Windows SDK）
- Cargo 镜像：rsproxy.cn
- npm 镜像：npmmirror.com

## 八、已注册命令速查（12 个，阶段 3 无新增）

```
group:    list_groups / create_group / update_group / delete_group
project:  list_projects(group_id?) / get_project / create_project / update_project / delete_project
process:  start_project(id) / stop_project(id) / restart_project(id)
```

阶段 5 拟新增：`list_running` / `probe_status(id)`

## 九、前端架构（阶段 3 产出）

```
src/
├── types/                  # TS 类型对齐 Rust serde（snake_case）
│   ├── group.ts            #   Group / GroupInput / GroupUpdate
│   └── project.ts          #   Project / ProjectInput / StartResult / ProjectType
├── api/                    # Tauri invoke 薄包装（统一错误解析）
│   ├── invoke.ts           #   invokeCmd<T>() + errMsg()
│   ├── group.ts            #   listGroups/createGroup/updateGroup/deleteGroup
│   ├── project.ts          #   listProjects/getProject/createProject/updateProject/deleteProject
│   └── process.ts          #   startProject/stopProject/restartProject
├── stores/                 # Pinia
│   ├── group.ts            #   groups[] + CRUD + safe() 包装
│   └── project.ts          #   projects[] + CRUD + start/stop/restart + runningIds 本地跟踪
├── components/project/
│   ├── StatusBadge.vue     #   运行态徽标（运行中/已停止）
│   ├── ActionBar.vue       #   启动/停止/重启/编辑/删除按钮组
│   ├── ProjectCard.vue     #   项目卡片（信息展示 + 抛事件）
│   └── ProjectFormDialog.vue # 新建/编辑 Dialog + 目录选择
└── views/
    ├── ProjectList.vue     #   分组 tab + 项目卡片网格 + 启停编排
    └── GroupManage.vue     #   分组增删改 + 上下移排序
```

**IPC 参数约定**（Tauri v2）：
- 顶层命令参数名用 camelCase（Tauri 自动转 snake_case）：`groupId` / `id` / `input`
- 嵌套结构体字段名用 snake_case（对齐 Rust serde，不转换）：`group_id` / `start_cmd` / `expected_ports`

## 十、文件清单

- 阶段 0 产出：详见 `docs/progress/00-scaffold.md`
- 阶段 1 产出：详见 `docs/progress/01-db-layer.md`
- 阶段 2 产出：详见 `docs/progress/02-process-core.md`
- 阶段 3 产出：详见 `docs/progress/03-project-ui.md`
