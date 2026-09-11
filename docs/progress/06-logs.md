# 阶段 6 - 日志流完成总结

> **后续修订（2026-09-11）**：本文件是阶段 6 的历史记录，下述设计已多次演进 ——
> 历史分页与日期分片已移除（改为每次启动/构建 truncate 单文件）；日志目录由
> 项目名改为**项目 id**；退订由「GC 自动退订」改为显式 `unsubscribe_log`。
> **当前设计以 [ADR-006](../adr/ADR-006-日志订阅生命周期与隔离.md) 为准。**

> 完成：实时 tail（ipc::Channel 推送）+ 历史分页读取 + 日志日期列举 + 清空日志 + 项目详情页日志面板
> 新增 4 个 Rust 命令、4 个 Rust 文件（logs 模块）、5 个前端文件，改造 3 个前端文件

## 产出物

### 后端（Rust）

**新增 `src-tauri/src/logs/` 模块**（4 文件，按 docs/10 目录规范）

- `mod.rs` — 模块入口 + 数据类型
  - `LogChunk { offset: u64, text: String }`（实时推送块）
  - `LogHistoryPage { total_size: u64, data: String, next_offset: Option<u64> }`（分页结果）
- `paths.rs` — 日志路径计算与日期列举
  - 复用 `spawn::sanitize_name`（提升为 pub）拼目录名，保证读写同路径
  - `project_log_dir` / `date_log_path` / `today` / `list_log_dates`（扫 `*.log`，过滤非 8 位纯数字，降序）
  - 3 个单元测试（路径布局 / 目录不存在返回空 / 降序+干扰文件过滤）
- `reader.rs` — 历史分页读取
  - `read_history(log_path, offset, limit) -> LogHistoryPage`：seek + 读 limit 字节，`next_offset` 标记下一页
  - 边界：文件不存在/offset 越界 → 空 page
  - 4 个单元测试（缺文件 / 全量读 / 分页有下一页 / offset 越界）
- `tail.rs` — 实时 tail task（核心）
  - `spawn_tail_task(log_path, on_event: Channel<LogChunk>)`：tokio 后台任务，detach
  - 起点策略：**从 offset 0 推送今日全部已有内容**，再 250ms 轮询增量
  - 截断/轮转检测：`file_size < last_offset`（clear_log 或外部清空）→ 重置 offset=0 重读全量
  - **自动退订**（ADR-002）：`on_event.send()` 返回 Err（前端 Channel GC）→ task break 退出
  - 文件未就绪重试：项目刚 spawn 文件可能没落盘，最多重试 10s
  - 1 个单元测试（增量读取 + 截断重读的文件 IO 逻辑）

**改造 `src-tauri/src/process/spawn.rs`**
- `sanitize_name` 从 `fn` 提升为 `pub fn`（跨模块复用，保证读写路径一致）

**新增 `src-tauri/src/commands/log.rs`** — 4 命令薄层（命名对齐 docs/03-命令清单.md §六）
```rust
subscribe_log(app, project_id, on_event: Channel<LogChunk>) -> AppResult<()>
read_log_history(app, project_id, date, offset, limit) -> AppResult<LogHistoryPage>
list_log_dates(app, project_id) -> AppResult<Vec<String>>
clear_log(app, project_id, date: Option<String>) -> AppResult<()>
```
- `subscribe_log`：取 project → 当日 log_path → `spawn_tail_task` → 立即返回 Ok(())（task 后台运行）
- `read_log_history`：单页字节上限 256KB（防超大单页）
- `clear_log`：`date=None` 清当天；truncate 保留文件；tail task 下次轮询自动检测重读

**改造 `src-tauri/src/lib.rs`** — 注册 4 命令（13 → 17），`mod logs`
**改造 `src-tauri/src/commands/mod.rs`** — `pub mod log`

### 前端（Vue/TS）

**新增 `src/types/log.ts`** — `LogChunk` / `LogHistoryPage` 类型（对齐 Rust serde）

**新增 `src/api/log.ts`** — 4 命令包装
- `subscribeLog(projectId, onEvent: Channel<LogChunk>)`（用 `@tauri-apps/api/core` 的 Channel）
- `readLogHistory` / `listLogDates` / `clearLog`

**新增 `src/stores/log.ts`** — 日志面板状态机
- state：`mode`（live/history）、`lines`（累积文本）、`loading`、`dates`、`historyDate`、`historyOffset`、`historyHasMore`
- `startLive(projectId)`：创建 Channel，`onmessage` 追加 chunk.text；持有 channelRef 防 GC；幂等（同项目不重复订阅）
- `stopLive()`：channelRef 置 null → GC → 后端自动退订
- `loadHistory(projectId, date, reset)`：分页读取，reset 替换 / 非追加
- `loadNextPage` / `fetchDates` / `clear` / `reset`

**重写 `src/views/ProjectDetail.vue`**（替换会报错的占位页）
- 顶部项目信息卡：名称/类型/路径/启动命令/端口 + StatusBadge（复用）+ 运行时 MetricsBar（复用）
- 日志面板：
  - 模式切换 `el-radio-group`（实时 / 历史）
  - 实时：`onMounted` startLive，`watch(lines)` 自动滚底，清空按钮
  - 历史：`el-select` 选日期（来自 list_log_dates）+「加载更多」分页
- 生命周期：`onMounted` getProject + startLive；`onBeforeUnmount` stopLive + reset

**改造 `src/views/ProjectList.vue`**
- 引入 `useRouter`
- 新增 `openDetail(p)` → `router.push({ name: 'ProjectDetail', params: { id: p.id } })`
- ProjectCard 绑定 `@open="openDetail(p)"`（卡片点击进入详情）

## 验证结果

- ✅ `cargo check`：通过（5 个既有预留字段 warning，无新增）
- ✅ `cargo test`：29 通过（含阶段 6 新增 8 个 logs 单测：paths×3 / reader×4 / tail×1）
- ✅ `vue-tsc --noEmit`：0 错误
- ✅ `vite build`：成功（ProjectDetail chunk 6.05kB）

## 关键技术决策

### 1. ipc::Channel 推送（ADR-002 落地）
用 `tauri::ipc::Channel<LogChunk>` 而非 Event 系统：
- 前端 Channel GC → `send()` 失败 → tail task 自动退出，无需显式 unsubscribe 命令
- 一对一绑定，类型安全（编译期 `Channel<LogChunk>`）
- 前端 store 持有 channel 引用防 GC，离开详情页置 null 触发退订

### 2. 实时起点 = 今日全部（offset 0）
从日志文件开头推送当日全部已有内容，再实时追加新行。
能完整看到项目启动日志，实现最简单（seek 到 0 即可）。不做「最近 N 行」的回溯截断。

### 3. 固定 250ms 轮询（不引 notify）
tail task 用固定间隔轮询文件增量，不引入 `notify` crate（额外依赖 + 跨平台差异）。
MVP 场景 250ms 延迟足够，实现简单可靠。

### 4. 截断/轮转自动重读
`clear_log` truncate 文件后，tail task 下次轮询检测到 `file_size < last_offset`，
自动重置 offset=0 重读全量。无需额外的截断通知机制，clear 与 tail 解耦。

### 5. 跳过 log_ref 表（MVP 扫文件系统）
`list_log_dates` 直接扫 `{logs_root}/{name}/` 目录读 `YYYYMMDD.log` 文件名，
不维护 DB 的 log_ref 表（文档 §02 允许 MVP 跳过）。后续若需索引可补。

### 6. 命令命名对齐命令清单
采用 `subscribe_log` / `read_log_history` / `list_log_dates` / `clear_log`
（docs/03-命令清单.md §六），非 CURRENT.md 早期草案的 `tail_logs`/`read_logs`。

## tail 数据流

```
子进程 stdout/stderr → append 落盘 {logs_root}/{name}/{YYYYMMDD}.log
                                          │
subscribe_log(projectId, onEvent: Channel)│
   │ 取 project → date_log_path(今日)
   └─ spawn_tail_task(log_path, channel)
        │
        ├─ open 文件（未就绪重试 ≤10s）
        ├─ push_all_available(offset=0) → 首推今日全部
        └─ loop (250ms):
             ├─ metadata → file_size
             ├─ size < offset? → 重置 0 重读（截断）
             ├─ size > offset? → push_all_available(增量)
             └─ send() Err? → break（Channel GC，自动退订）
        │
   channel.send(LogChunk{offset, text}) ──▶ 前端 channel.onmessage
                                              └─ store.lines += chunk.text
```

## 项目详情页布局

```
┌─────────────────────────────────────────────┐
│  ← 返回                                      │
├─────────────────────────────────────────────┤
│  HR后端  [SpringBoot]  [●运行中]            │
│  路径  D:\Project\HR                        │
│  启动  mvn spring-boot:run                   │
│  端口  8080                                  │
│  ┌─ MetricsBar（运行中显示）─┐              │
│  │ CPU 3.2% · 内存 412MB · 8080 监听中      │
│  └────────────────────────────┘             │
├─────────────────────────────────────────────┤
│  [实时][历史]   (历史: [日期▾] [加载更多])   │
│                          1234 行   [清空]   │
│  ┌──────────────────────────────────────┐  │
│  │   .   ____          _            __  │  │
│  │  /\\ / ___'_ __ _ _(_)_ __  __ _ \ \ │  │
│  │ ( ( )\___ | '_ | '_| | '_ \/ _` | │  │  │
│  │  \\/  ___)|-_) | | | | || (_| |  | |  │  │
│  │   '  |____| .__|_| |_|_| |_\__, | │  │  │
│  │  =========|_|==============|___/=_]  │  │
│  │  :: Spring Boot ::  (v3.2.0)         │  │
│  │  ...                                 │  │
│  └──────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

## 已知限制 / 后续

1. **实时模式仅当日**：tail 绑定当天日志文件，跨天后不会自动切换（需重新订阅）
2. **历史按字节分页**：不按行对齐，翻页边界可能截断一行（前端 `<pre>` 仍可读）
3. **清空仅 truncate**：保留文件不删，磁盘空间即时回收；未做日志轮转/过期清理（风险 R2 后续）
4. **tail task 不跨重启**：应用重启后需重新进入详情页订阅（已落盘内容仍在）

## 下一步 → 阶段 7（构建命令 + 一键发布）

1. 后端 `build_project(id, on_event: Channel<BuildEvent>)`：执行 build_cmd，stdout/stderr 实时推送
2. 前端一键发布编排：stop → build → start（前端组合调用）
3. 构建输出复用 Channel 机制（ADR-002 同样适用）
