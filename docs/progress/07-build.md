# 阶段 7 - 构建命令 + 一键发布完成总结

> 完成：build_project 命令（piped stdout/stderr 实时推 Channel）+ 一键发布编排（stop→build→start）+ 构建日志对话框
> 新增 1 个 Rust 命令、1 个 Rust 文件（process/build 模块）、3 个前端文件，改造 4 个前端文件
> **MVP 全部功能完成**（项目注册 / 启停 / 重启 / 构建 / 发布 / 监控 / 日志）

## 产出物

### 后端（Rust）

**新增 `src-tauri/src/process/build.rs`** —— 构建执行核心
- `BuildEvent` 枚举（`#[serde(tag="kind", content="data", rename_all="snake_case")]`）：
  `Stdout(String)` / `Stderr(String)` / `Exit(i32)`，序列化为 `{kind, data}` 对齐命令清单
- `BuildResult { exit_code: i32, duration_ms: u64 }`
- `run_build(project, on_event: Channel<BuildEvent>) -> AppResult<BuildResult>`：
  1. 校验 `build_cmd` 非空（None/空白 → `AppError::Process`）
  2. `cmd /C <build_cmd>`，`current_dir(project.path)`，`stdout/stderr = Stdio::piped()`，`stdin(null)`，`CREATE_NO_WINDOW`
  3. spawn → 取 child → 记起始 `Instant`
  4. **管道并发读取**：`child.stdout` / `child.stderr` 各 `tokio::spawn` 一个 `pump_pipe` task，4KB buf 读 → `on_event.send(Stdout/Stderr)`（send 失败=前端 GC → break，child 由 kill_on_drop 回收）
  5. `child.wait().await` → exit code → `on_event.send(Exit)`
  6. 返回 `BuildResult{exit_code, duration_ms}`
- 3 个单元测试（BuildEvent 序列化 tag/content 格式 / BuildResult snake_case / build_cmd 空校验）

**与 `start_project`（spawn 模块）的本质区别**：
| | start_project | build_project |
|---|---|---|
| 进程性质 | 常驻（手动 stop） | 一次性（跑完自退） |
| stdout/stderr | 追加写日志文件（Stdio::from File） | piped 实时推 Channel |
| 进程托管 | Job Object + registry | kill_on_drop 兜底 |
| 退出码 | 不关心（探测靠 sysinfo） | 通过 BuildResult 返回 |
| 复用 ADR-002 | 阶段 6 tail task 读日志文件推 Channel | 直接 piped 推 Channel |

**改造 `src-tauri/src/process/mod.rs`** —— `pub mod build;` 导出 `BuildEvent`/`BuildResult`

**新增 `src-tauri/src/commands/process.rs` 的 `build_project` 命令**
```rust
#[tauri::command]
pub async fn build_project(app, id: i64, on_event: Channel<BuildEvent>) -> AppResult<BuildResult>
```
- 取 project → `run_build` → 返回 BuildResult
- 命令会 await 到构建结束（Channel 流式输出中途到达）

**改造 `src-tauri/src/lib.rs`** —— 注册 `build_project`（命令 17→18）

### 前端（Vue/TS）

**改造 `src/types/project.ts`**
- `BuildEvent`：discriminated union（`{kind:'stdout',data:string}` | `{kind:'stderr',data:string}` | `{kind:'exit',data:number}`）
- `BuildResult { exit_code, duration_ms }`

**改造 `src/api/process.ts`** —— 加 `buildProject(id, onEvent: Channel<BuildEvent>)`

**新增 `src/stores/build.ts`** —— 构建状态机
- state：`projectId`、`lines: BuildLine[]`（带 stdout/stderr 标记）、`running`、`exitCode`、`durationMs`、`error`
- `startBuild(id)`：创建 Channel，`onmessage` 按 kind 分流（exit→exitCode，stdout/stderr→lines）；`channelRef` ref 持引用防 GC；返回 `Promise<BuildResult|null>`（构建结束 resolve，供编排等待）
- `stopBuild()`：channelRef 置 null → GC → 后端 send 失败退出
- `reset()` / `isSucceeded()`

**改造 `src/components/project/ActionBar.vue`** —— 加「构建」「发布」按钮
- 新增 `canBuild` prop（build_cmd 为空时禁用 + title 提示）
- emit `build` / `deploy`

**改造 `src/components/project/ProjectCard.vue`**
- computed `canBuild`（build_cmd 非空）
- ActionBar 透传 `:can-build` + `@build` / `@deploy`

**新增 `src/components/project/BuildDialog.vue`** —— 构建日志对话框
- 深色输出区（仿终端），stderr 红色着色
- 底部状态条四态：running（转圈蓝）/ succeeded（绿√）/ failed（红×）/ error（红，命令错误）
- 自动滚到底（watch lines.length）
- 关闭即 reset 释放 Channel（GC → 后端读取 task 退出 → kill_on_drop 回收 child）

**改造 `src/views/ProjectList.vue`** —— 构建与一键发布编排
- `handleBuild(p)`：弹 BuildDialog → `buildStore.startBuild(p.id)`
- `handleDeploy(p)`：确认 → stop（若运行）→ build（等结果）→ start；任一步失败中止并提示
  - 编排逻辑放前端（IPC 边界原则 §4.2：Rust 只做原子操作，组合在前端）
- `onBeforeUnmount` 加 `buildStore.reset()`

## 验证结果

- ✅ `cargo check`：通过（6 个既有预留字段 warning，无新增）
- ✅ `cargo test`：32 通过（含阶段 7 新增 3 个 build 单测）
- ✅ `vue-tsc --noEmit`：0 错误
- ✅ `vite build`：成功（ProjectList chunk 15.74kB）

## 关键技术决策

### 1. piped stdout/stderr 实时推 Channel（ADR-002 复用）
构建与日志 tail 用同一 Channel 机制，但数据源不同：
- 日志 tail：读已落盘的日志文件（spawn 写文件 → tail 读文件）
- 构建：直接 piped 子进程 stdout/stderr → 读管道 → 推 Channel（不落盘）
构建是短命令，输出无需持久化，实时推完即弃。

### 2. 管道并发读取（防死锁）
stdout/stderr 必须各起独立 task 并发读。若串行先读 stdout，stderr 填满 OS 管道缓冲区（~64KB）
会导致子进程 write 阻塞 → 整体死锁。`pump_pipe` 抽象 + 两 task 并发是标准解法。

### 3. 一次性进程不入 registry / 不用 Job Object
构建跑完自退，无需手动停止，故不入 ProcessRegistry（那是常驻进程用的）、不挂 Job Object
（那是杀整树用的）。仅靠 `kill_on_drop(true)` 兜底：前端关 Channel → 读取 task 退出 → child drop → kill。

### 4. 一键发布编排放前端（IPC 边界原则）
stop → build → start 是业务编排，按 §4.2「不在 Rust 里写复杂业务分支」原则放前端。
前端用 `buildStore.startBuild` 返回的 Promise 等待构建结束，成功才 start，失败中止。
Rust 只提供原子操作（stop_project / build_project / start_project）。

### 5. BuildEvent 用 serde tag/content（discriminated union）
`#[serde(tag="kind", content="data")]` 序列化为 `{kind, data}`，前端用 TS discriminated union
精确收窄 data 类型（exit 是 number，stdout/stderr 是 string）。比扁平 enum + 联合字段类型更安全。

## build 数据流

```
build_project(id, onEvent: Channel) 入口
   │ 取 project → 校验 build_cmd
   └─ run_build(project, channel)
        ├─ cmd /C <build_cmd> (piped stdout/stderr)
        ├─ spawn → child
        ├─ tokio::spawn pump_pipe(stdout) ─┐
        ├─ tokio::spawn pump_pipe(stderr) ─┤  并发读 4KB 块
        │                                  │
        │   on_event.send(Stdout/Stderr) ◀─┘   send 失败 → break（Channel GC）
        │
        ├─ child.wait().await → exit code
        ├─ on_event.send(Exit(code))
        └─ return BuildResult{exit_code, duration_ms}
                                        │
   channel.send(BuildEvent) ──▶ 前端 onmessage ──▶ store.lines / exitCode
```

## 一键发布编排（前端）

```
handleDeploy(p) 确认
   │
   ├─ 1. stop（若 isRunning）─── 失败 → 中止「停止失败」
   ├─ 2. build（await startBuild）── exit_code≠0 → 中止「构建失败」
   └─ 3. start ─────────────────── 失败 → 「构建成功但启动失败」
        成功 → 「发布完成」
```

## ActionBar 按钮布局

```
[启动/停止] [重启] [构建] [发布] [编辑] [删除]
                       ↑        ↑
                  canBuild 控制  primary plain
                  （build_cmd 空则禁用 + title「未配置构建命令」）
```

## 已知限制 / 后续

1. **同时只支持一个构建面板**：store 是单例，handleDeploy/handleBuild 复用同一 BuildDialog
2. **构建中关闭对话框放弃订阅**：依赖 kill_on_drop 回收 child，不保证立即终止（构建进程可能短暂残留）
3. **发布中无法取消**：编排是 await 链，一旦开始跑到结束或失败；如需取消需引入 AbortSignal
4. **构建日志不持久化**：piped 不落盘，关闭对话框后丢失；如需归档可后续接日志文件方案

## MVP 完成情况

开发计划 §八 MVP 清单全部就位：
- ✅ 项目注册（含分组）
- ✅ 启动 / 停止 / 重启
- ✅ 构建 / 一键发布（本阶段）
- ✅ PID / CPU / 内存监控
- ✅ 端口监控
- ✅ 实时日志

**完成后即可替代**：多个 IDEA Run Configuration、PM2、部分 Process Explorer 使用场景。
后续 P1/P2（Git 状态、IDE 打开、Docker Compose、AI 进程识别、依赖启动）为迭代增强。
