# 阶段 3 - 项目注册 UI 完成总结

> 完成：TS 类型 + API 层 + Pinia stores + 项目卡片组件 + 列表页 + 分组管理页
> 对接已有 12 个后端命令（4 group + 5 project + 3 process），无新增 Rust 代码

## 产出物

### types/（2 文件，对齐 Rust serde snake_case）
- `types/group.ts` — `Group` / `GroupInput` / `GroupUpdate`
- `types/project.ts` — `Project` / `ProjectInput` / `StartResult` / `ProjectType`
  - `PROJECT_TYPE_OPTIONS`：表单下拉用（5 种类型）
  - `PROJECT_TYPE_LABELS`：类型→中文标签

### api/（4 文件，invoke 薄包装）
- `api/invoke.ts` — `invokeCmd<T>()` 泛型包装 + `errMsg()` 统一错误取消息
- `api/group.ts` — listGroups / createGroup / updateGroup / deleteGroup
- `api/project.ts` — listProjects / getProject / createProject / updateProject / deleteProject
- `api/process.ts` — startProject / stopProject / restartProject

### stores/（2 文件，Pinia）
- `stores/group.ts` — groups[] 缓存 + add/patch/remove（本地即时同步）+ `safe()` 包装
- `stores/project.ts` — projects[] + CRUD + start/stop/restart + **runningIds 本地 Set**
  - start 成功 → 加 id 到 runningIds + 更新本地 last_pid/last_start_time
  - stop 成功 → 移出 runningIds + last_pid 置 null
  - `onGroupDeleted(gid)`：分组删除时清理项目的 group_id 引用

### components/project/（4 组件）
- `StatusBadge.vue` — 运行态徽标，运行中带脉动绿点 + box-shadow 光晕
- `ActionBar.vue` — 启动/停止（互斥）+ 重启 + 编辑 + 删除（运行中禁删）
- `ProjectCard.vue` — 信息卡片：名称/类型 tag/状态/路径/启动命令/端口/PID + ActionBar
- `ProjectFormDialog.vue` — 新建/编辑 Dialog
  - 表单：名称/类型/分组/目录/启动命令/构建命令/预期端口/启用
  - 目录选择：前端直连 `@tauri-apps/plugin-dialog` 的 `open({directory:true})`
  - 端口输入逗号分隔，提交时拆数组 trim
  - create/edit 复用，edit 预填字段；defaultGroupId 支持分组 tab 内新建预选

### views/（2 页面）
- `ProjectList.vue` — 主页
  - 顶部：分组 tab（全部/未分组/各分组，带项目数 badge）+ 新建按钮
  - 网格：auto-fill 响应式卡片布局（minmax 340px）
  - 编排：所有 store 操作集中于此，卡片无状态
  - busyIds Set 跟踪每个项目进行中的启停操作
- `GroupManage.vue` — 分组管理
  - 顶部：新建分组表单（inline + 回车提交）
  - 表格：上移/下移改 order、行内重命名（双击或按钮）、项目数 badge、删除（确认）

## 验证结果

- ✅ `vue-tsc --noEmit`：0 错误
- ✅ `vite build`：成功（ProjectList 11.67kB / GroupManage 4.18kB 分块）
- ✅ `cargo check`：通过（仅 2 个预留字段 warning，与阶段 2 一致）

## 关键技术决策

### 1. 目录选择用前端 dialog 插件，不加 Rust 命令
CURRENT.md 原计划加 `pick_directory` 命令，但 `@tauri-apps/plugin-dialog` 的 `open({directory:true})`
已满足需求，且 `capabilities/default.json` 的 `dialog:default` 权限已含 `allow-open`。
无需新增后端代码，更简单。

### 2. 卡片组件无状态，编排集中在 ProjectList
ProjectCard 只展示 + 抛事件（start/stop/restart/edit/delete/open），
真正的 store 操作由父组件 ProjectList 统一处理。
好处：卡片易复用、易测试，状态变更路径单一。

### 3. runningIds 本地 Set（阶段 3 → 5 过渡）
阶段 3 尚无「批量查询运行中项目」的后端命令（ProcessRegistry.running_ids 已就绪但未暴露）。
前端用本地 Set 记忆「本 session 内被前端启动且未停止」的 project_id。
**限制**：重启应用后所有项目显示「已停止」（符合实际，Job Object 回收进程）；
应用外被 kill 不会即时更新。**阶段 5** 加 `list_running`/`probe_status` 命令替换。

### 4. busyIds 跟踪启停进行中
避免用户对同一项目连续点启停导致竞态。
按 project id 的 Set，操作期间禁用该卡片所有按钮。

### 5. 删除前校验运行态
handleDelete 先检查 isRunning，运行中直接 ElMessage.warning 拒绝，
不依赖后端报错（更友好的 UX）。

## IPC 参数大小写约定（Tauri v2）

**顶层命令参数**用 camelCase（Tauri IPC 自动转 snake_case 匹配 Rust 参数名）：
```ts
invoke('list_projects', { groupId })       // → Rust group_id: Option<i64>
invoke('get_project', { id })              // → Rust id: i64
invoke('create_project', { input })        // → Rust input: ProjectInput
```

**嵌套结构体字段**用 snake_case（与 Rust serde 一致，IPC 不转换）：
```ts
{ name, group_id, type, path, start_cmd, build_cmd, expected_ports, enabled }
//   ↑ 对齐 Rust ProjectInput 的 serde 字段名
```

## 已知限制 / 阶段 4-5 待补

1. **运行态非持久**：见上述第 3 点，阶段 5 用真实探测替换
2. **项目详情页**：仍是 PlaceholderPage，阶段 4/5/6 实现（监控 + 日志）
3. **构建命令 / 一键发布**：阶段 7 实现（ProjectCard 已预留，ActionBar 暂未加按钮）
4. **分组 tab 项目数**：前端统计（projects[] filter），数据量小无需每次查 DB

## 下一步 → 阶段 5（监控面板）

1. 后端加命令：`list_running`（返 `Vec<{id, pid, started_at, log_path}>`）+ `probe_status(id)`（CPU/内存/端口）
2. 前端 store 用真实探测替换 runningIds 本地 Set
3. 定时（3s）轮询运行态，卡片显示 CPU/内存/端口监听
4. 状态机扩展：Running / Running(Abnormal)（进程在但端口未监听）/ Stopped
