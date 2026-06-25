# 阶段 5 - 监控面板完成总结

> 完成：后端 probe_statuses 命令（sysinfo 进程树聚合 + netstat2 端口归属校验）+ 前端 3s 轮询 + 三态状态机 + 实时指标卡片
> 新增 1 个 Rust 命令、3 个 Rust 模块项、4 个前端文件，改造 4 个前端文件

## 产出物

### 后端（Rust）

**新增 `src-tauri/src/process/monitor.rs`** —— 监控核心逻辑
- `HealthStatus` 枚举（serde snake_case）：`stopped` / `running` / `running_abnormal`
- `PortStatus` 结构：`{ port, listening, owned }`
- `ProjectStatus` 结构：`{ project_id, health, pid, cpu_percent, memory_bytes, ports, started_at }`
- `probe_one(project_id, snapshot, expected_ports, system, sockets)`：
  - `collect_tree` 收集全树 PID → CPU/内存整树求和（避免 npm/mvn 包装进程显示 0）
  - 端口探测：netstat2 查 TCP LISTEN + 占用者 PID 归属校验（防 R3）
  - 健康判定：无预期端口→running；否则要求全部 listening&&owned，否则 abnormal
- `collect_tcp_sockets()`：一次 `get_sockets_info` 查全表，所有项目复用
- 3 个单元测试（聚合空进程树/非数字端口/无预期端口健康判定）

**改造 `state.rs`** —— 加持久化 sysinfo System
- `Inner.system: Mutex<sysinfo::System>`：CPU 准确性要求两次 refresh 间隔，全程复用同一实例形成基线
- `system()` 访问器

**改造 `commands/process.rs`** —— 新增 `probe_statuses` 命令
```rust
#[tauri::command]
pub async fn probe_statuses(app, ids: Option<Vec<i64>>) -> AppResult<Vec<ProjectStatus>>
```
- 解析 id 列表（None/空→全部 running_ids；去重+过滤非运行）
- 批量 SQL 取 expected_ports（避免 N+1）
- refresh 系统进程（cpu 基线）+ 取 TCP 全表一次 + 逐项目 probe（持 system 锁，短暂无 await）
- 辅助：`fetch_expected_ports`（HashMap 批量）、`refresh_system`

**改造 `lib.rs`** —— 注册 `probe_statuses`
**改造 `registry.rs` / `tree.rs`** —— 移除 `#[allow(dead_code)]`（snapshot/running_ids/collect_tree 现已被使用）

### 前端（Vue/TS）

**新增 `src/types/monitor.ts`**
- `HealthStatus` / `PortStatus` / `ProjectStatus` 类型（对齐 Rust serde）
- `HEALTH_LABELS` 状态→中文标签、`formatBytes` 字节格式化

**新增 `src/api/monitor.ts`** —— `probeStatuses(ids?)`

**改造 `src/stores/project.ts`** —— 真实探测替换本地 runningIds
- 新增 `statuses = ref<Record<number, ProjectStatus>>({})`
- 新增 `probeNow()`（重入安全）/ `startPolling()`（3s，幂等）/ `stopPolling()`
- start/stop/restart 成功后 `probeNow()` 立即刷新（乐观更新 + 真实探测兜底）
- 新增 `getStatus(id)` / `getHealth(id)` / `isRunning(id)`
- 删除旧的 `runningIds` Set + `setRunning`

**改造 `src/components/project/StatusBadge.vue`** —— 三态
- prop 从 `running: boolean` 改为 `health: HealthStatus`
- running(绿脉动) / running_abnormal(橙+Warning 图标) / stopped(灰)

**新增 `src/components/project/MetricsBar.vue`** —— 运行时指标条
- CPU%（1 位小数）/ 内存（formatBytes）/ 端口徽标三态（监听中/被占用/未监听）

**改造 `src/components/project/ProjectCard.vue`**
- prop 从 `running: boolean` 改为 `status?: ProjectStatus | null`
- computed `health` / `running` / `pid`（优先 status.pid 回退 last_pid）
- 运行中时在 meta 下方渲染 MetricsBar
- StatusBadge 用 `:health`、ActionBar 用 computed `running`

**改造 `src/views/ProjectList.vue`**
- `onMounted`：`loadAll()` + `startPolling()`
- `onBeforeUnmount`：`stopPolling()`
- ProjectCard 绑定 `:status="projectStore.statuses[p.id] ?? null"`

## 验证结果

- ✅ `cargo check`：通过（4 个预留字段 warning，详见 CURRENT.md §三）
- ✅ `cargo test`：21 通过（含阶段 5 新增 3 个 monitor 单测）
- ✅ `vue-tsc --noEmit`：0 错误
- ✅ `vite build`：成功（ProjectList chunk 13.48kB）

## 关键技术决策

### 1. 指标聚合整树（而非仅根进程）
npm run dev → node、mvn spring-boot:run → java：真正的资源消耗在工作子进程。
只看根进程会显示 0。用已有的 `collect_tree` 聚合根+所有后代 PID 的 CPU/内存，最准确。

### 2. 端口归属校验（防风险 R3）
端口 Listen 时，再校验占用者 PID（netstat2 的 `associated_pids`）是否在本项目进程树内。
若 8080 被别的程序占，本项目 `owned=false` → 标 abnormal，不误报端口正常。

### 3. 持久化 sysinfo System（CPU 基线）
sysinfo `cpu_usage()` 需两次 refresh（间隔 > MINIMUM_CPU_UPDATE_INTERVAL）才准确。
AppState 持久化 System，3s 轮询每次 `refresh_processes` 形成基线，首次 probe 后 cpu 即有意义。

### 4. 固定 3s 轮询（不做启动宽限期）
SpringBoot 等慢启动项目，端口就绪前显示「运行中(异常)」，就绪后自动恢复。
用户能直观看到启动进度，逻辑最简单（无需额外状态机和计时）。

### 5. netstat2 全表查一次复用
`get_sockets_info` 较重（遍历系统全表），一次调用后对所有项目复用同一 sockets slice，
避免 N 次 netstat 调用。

## probe_statuses 命令流程

```
probe_statuses(ids?) 入口
  │
  ├─ ids 为空? → registry.running_ids() 取全部
  ├─ 去重 + 过滤非运行（registry.contains）
  │
  ├─ fetch_expected_ports(批量 SQL) → HashMap<id, Vec<String>>
  │
  ├─ system.lock()
  │   ├─ refresh_processes(All, remove_dead=true)  ← cpu 基线
  │   └─ collect_tcp_sockets()  ← netstat2 一次
  │
  ├─ for id in to_probe:
  │     ├─ registry.snapshot(id) → ProcessSnapshot
  │     └─ probe_one(id, snap, ports, system, sockets)
  │           ├─ collect_tree(snap.pid) → HashSet<u32>
  │           ├─ aggregate_tree → (cpu_sum, mem_sum)
  │           ├─ check_port × N → Vec<PortStatus>
  │           └─ health 判定
  │
  └─ 返回 Vec<ProjectStatus>（前端不在结果中的 id → stopped）
```

## 状态机

| 状态 | 触发条件 | UI |
|---|---|---|
| stopped | registry 无此项目 | 灰色「已停止」 |
| running | 进程在 且（无预期端口 或 全部端口 listening&&owned） | 绿色脉动「运行中」 |
| running_abnormal | 进程在 但存在端口未监听/被占用 | 橙色+警告「运行中(异常)」 |

## 已知限制 / 后续

1. **cpu 首次 probe 可能偏低**：第一次 refresh 后 cpu_usage 基于「构造以来的平均」，3s 后趋准
2. **netstat2 失败降级**：get_sockets_info 失败时返回空表，端口全显示未监听（不中断整体 probe）
3. **轮询仅在 ProjectList 挂载时运行**：切到分组管理页会 stopPolling；详情页（阶段 6）需自管轮询
4. **未做 Starting/Stopping 中间态**：保持三态简化；启停操作期间按钮 busy 跟踪已覆盖交互

## 下一步 → 阶段 6（日志流）

1. 后端 `tail_logs(projectId, channel)`：ipc::Channel 推送实时增量（ADR-002）
2. 后端 `read_logs(projectId, date?, offset?, limit?)`：历史分页
3. 前端日志面板 + ProjectDetail.vue 实现
