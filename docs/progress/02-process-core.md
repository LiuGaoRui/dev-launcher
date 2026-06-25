# 阶段 2 - 进程托管核心完成总结

> 完成：spawn + Windows Job Object + ProcessRegistry + start/stop/restart 命令

## 产出物

### process 模块（新增 5 文件）
- `src-tauri/src/process/mod.rs` — 模块入口 + `StartResult` 结构体 + ProcessRegistry 重导出
- `src-tauri/src/process/job_object.rs` — `JobHandle`：Windows Job Object 的 unsafe FFI 封装
  - `new()`：CreateJobObjectW + 设 KILL_ON_JOB_CLOSE
  - `assign(raw_handle)`：AssignProcessToJobObject
  - `terminate()`：TerminateJobObject（杀整树）
  - `active_processes()`：QueryInformationJobObject 读活跃进程数（stop 轮询用）
  - RAII：drop 自动 CloseHandle → 触发 KILL_ON_JOB_CLOSE
  - 非 Windows 平台 `cfg` 占位
- `src-tauri/src/process/spawn.rs` — `spawn_command` + `log_file_path` + 项目名清洗
  - `cmd /C <start_cmd>` 执行，支持 &&、管道、重定向
  - stdout/stderr 两独立 append 句柄落盘同一日志文件
  - CREATE_NO_WINDOW 避免弹窗；kill_on_drop 兜底
- `src-tauri/src/process/registry.rs` — `ProcessRegistry`：`Mutex<HashMap<i64, RunningProcess>>`
  - insert/remove/contains/snapshot/running_ids/with_mut
  - `RunningProcess { child, job, pid, log_path, started_at }`
- `src-tauri/src/process/tree.rs` — `collect_descendants` / `collect_tree`（sysinfo，阶段 5 监控用）

### 改动
- `src-tauri/src/state.rs` — AppState 增加 `registry: ProcessRegistry` + `registry()` 访问器
- `src-tauri/src/commands/process.rs` — 3 个命令薄层
- `src-tauri/src/commands/mod.rs` — 注册 process 子模块
- `src-tauri/src/lib.rs` — 声明 `mod process`；invoke_handler 追加 3 命令
- `src-tauri/Cargo.toml` — windows features 修正（+Win32_Security, -ProcessStatus）；dev-dep 加 tempfile

### 已注册的 3 个新命令

| 命令 | 签名 | 行为 |
|---|---|---|
| `start_project` | `(app, id) -> StartResult` | spawn + Job + 入 registry + 更新 DB + 后台 wait task |
| `stop_project` | `(app, id) -> ()` | Job terminate → 轮询 ActiveProcesses 归零（5s）→ 兜底 kill → 移除 registry + 更新 DB |
| `restart_project` | `(app, id) -> StartResult` | stop（若运行）→ start |

`StartResult { root_pid: u32, log_path: String, started_at: String }`

## 验证结果

- ✅ `cargo check` 通过（仅 state.rs 的 data_dir 预留字段 warning）
- ✅ `cargo test`：18 单测全绿（阶段1 的 10 + 阶段2 的 8：registry 4 + spawn 2 + tree 2），2 集成测试 ignored
- ✅ `cargo test -- --ignored`：2 个真实 spawn/stop 集成测试通过
  - `job_terminate_kills_spawned_process`：spawn ping → assign → terminate → ActiveProcesses 归零 → child 退出
  - `job_active_processes_empty_after_child_exits`：echo 自然退出后 ActiveProcesses 归零
- ✅ `vite build` 成功

## 关键技术决策

### 1. CREATE_SUSPENDED 不可行 → spawn-then-assign
std/tokio 在 spawn 返回前关闭主线程句柄，无法 ResumeThread。改 spawn 后立即 Assign，微秒级竞态由 KILL_ON_JOB_CLOSE 兜底（详见 ADR-001 附录）。

### 2. JobHandle 用 Owned<HANDLE> RAII
drop 自动 CloseHandle 杀整树；HANDLE 含裸指针非 Send/Sync，显式 unsafe impl（经 Mutex 串行访问）。

### 3. stop 轮询策略
Job terminate → 轮询 ActiveProcesses 归零（最多 5s，每 100ms）→ 超时则 child.start_kill 兜底 → 移除 registry。

### 4. 后台 wait task 用 try_wait 轮询（非 child.wait().await）
不能持 registry 锁 await（会阻塞其他操作如 stop），故用 try_wait 每 500ms 轮询，进程退出后清理 registry + 更新 DB。

### 5. try_wait 延迟
Job 终止后 OS 已回收进程，但 tokio Child::try_wait 可能短暂返回 None，需轮询等待（集成测试验证）。

## 遇到的问题与解决

### 1. windows_core 导入路径
- **现象**：`unresolved import windows_core`
- **解决**：用 `windows::core::{Owned, PCWSTR}`（windows_core 是 windows 的内部 crate，通过 windows::core 访问）

### 2. Owned<HANDLE> 非 Send/Sync
- **现象**：`*mut c_void cannot be sent between threads safely`（无法入 Mutex/registry）
- **解决**：JobHandle 显式 `unsafe impl Send/Sync`（经 Mutex 串行访问，OS 句柄跨线程安全）

### 3. FFI 调用不能传 `&HANDLE`
- **现象**：`&HANDLE: Param<HANDLE, CopyType>` 不满足
- **解决**：HANDLE 是 CopyType，用 `handle.clone()` 传值（Owned 支持 clone，引用计数）

### 4. CreateJobObjectW 缺 Win32_Security feature
- **解决**：Cargo.toml 加 `Win32_Security`（即便传 None 安全属性也要此 feature）

### 5. 集成测试 try_wait 延迟
- **现象**：terminate + ActiveProcesses 归零后，child.try_wait() 仍返回 None
- **解决**：轮询 try_wait 直到 Some（最多 2s），而非单次断言

## 下一步 → 阶段 3（项目注册 UI）

1. 前端 `src/types/{group,project}.ts` — TS 类型对齐 Rust serde
2. `src/api/{invoke,group,project}.ts` — Tauri invoke 薄包装
3. `src/stores/{project,group}.ts` — Pinia
4. `src/views/ProjectList.vue` — 分组树 + 项目卡片
5. `src/components/project/{ProjectCard,StatusBadge,ActionBar}.vue`
6. `src/views/GroupManage.vue` + ProjectForm
7. `pick_directory` 命令（接 dialog 插件）
