//! cleaner 命令薄层：scan_dev_processes / kill_dev_processes / trim_dev_processes / get_system_memory
//! + 进程锁定 CRUD（list_cleaner_locks / add_cleaner_lock / remove_cleaner_lock / clear_cleaner_locks）。
//!
//! 内存清理器后端：扫描整机开发进程（java/node），智能分类 + 推荐清理目标，
//! 批量杀进程树、修剪工作集回收内存、查询系统内存概况。
//!
//! 与 commands/process.rs 的区别：process 管理本工具**托管**的项目进程；
//! cleaner 面向全系统**任意**开发进程（含 IDE 派生、孤立残留），用于释放内存。
//!
//! 两个正交动作：
//! - kill：杀进程树（taskkill /F /T），进程消失——用于孤立残留、泄漏的 dev server
//! - trim：修剪工作集（SetProcessWorkingSetSizeEx -1,-1），进程继续运行——用于 IDE 内存回收

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
use tauri::{AppHandle, Manager, Runtime};

use crate::db;
use crate::error::AppResult;
use crate::models::{CleanerLock, CleanerLockInput};
use crate::process::dev_scan::{self, DevProcInfo};
use crate::process::kill::kill_process_tree;
use crate::process::trim::trim_working_set;
use crate::services::CleanerLockService;

/// 构建开发进程扫描所需的 ProcessRefreshKind。
///
/// sysinfo 0.32 默认不采集 cmd/cwd/exe 等昂贵字段（返回空），必须显式请求。
/// 这对 dev_scan 至关重要——display_title / cmdline_summary / project_path
/// 全部依赖命令行参数，cwd 是项目路径的最后兜底。
fn dev_refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::new()
        .with_cmd(UpdateKind::Always)
        .with_cwd(UpdateKind::Always)
        .with_exe(UpdateKind::Always)
        .with_memory()
        .with_cpu()
}
use crate::state::AppState;

/// 批量杀进程的结果。
#[derive(Debug, Clone, Serialize)]
pub struct KillResult {
    /// 成功杀死的进程树数量
    pub killed: u32,
    /// 杀失败的进程树数量
    pub failed: u32,
    /// 因命中锁定表被跳过的进程树数量（后端纵深防御）
    pub skipped_locked: u32,
    /// 杀死前这些进程树的内存总和（字节，用于展示「已释放」）
    pub freed_bytes: u64,
}

/// 批量修剪工作集（内存回收）的结果。
#[derive(Debug, Clone, Serialize)]
pub struct TrimResult {
    /// 成功修剪的进程数量
    pub trimmed: u32,
    /// 修剪失败的进程数量（权限不足或进程已退出）
    pub failed: u32,
    /// 因命中锁定表被跳过的进程数量（后端纵深防御）
    pub skipped_locked: u32,
    /// 回收的物理内存总量（字节，回收前后 RSS 差值之和）
    pub freed_bytes: u64,
}

/// 系统内存概况。
#[derive(Debug, Clone, Serialize)]
pub struct SystemMemory {
    /// 物理内存总量（字节）
    pub total_bytes: u64,
    /// 已用（字节）
    pub used_bytes: u64,
    /// 可用 = total - used
    pub available_bytes: u64,
    /// 使用率（0-100）
    pub used_percent: f32,
}

/// 扫描全系统开发进程（java/javaw/node 等），返回智能分类后的列表。
///
/// 复用 AppState 持久化的 sysinfo System 实例（CPU 基线一致性）。
/// 排除本应用自身进程，避免误杀。
///
/// 前端在打开清理抽屉时调用 + 定时（约 5s）刷新。
#[tauri::command]
pub async fn scan_dev_processes<R: Runtime>(app: AppHandle<R>) -> AppResult<Vec<DevProcInfo>> {
    let state = app.state::<AppState>();
    let self_pid = sysinfo::get_current_pid().ok();

    // 刷新全系统进程（cpu + 内存 + 命令行 + cwd + exe）后扫描
    let mut processes = {
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, dev_refresh_kind());
        dev_scan::scan_dev_processes(&system, self_pid)
    };

    // 标注锁定态：按进程指纹匹配 cleaner_lock 表（与 PID 无关，进程重启后仍能命中）。
    // 指纹在扫描时已基于完整命令行算好（DevProcInfo.fingerprint）。
    // DB 读失败降级为全未锁定（不阻断扫描），下次轮询自动恢复。
    let pool = db::pool(&app);
    match pool {
        Ok(pool) => match CleanerLockService::locked_map(&pool).await {
            Ok(locked) if !locked.is_empty() => {
                for p in processes.iter_mut() {
                    if let Some(&id) = locked.get(&p.fingerprint) {
                        p.locked = true;
                        p.lock_id = Some(id);
                    }
                }
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("读取进程锁定表失败（本次扫描不标注锁定态）: {e}"),
        },
        Err(e) => tracing::warn!("获取数据库连接池失败（本次扫描不标注锁定态）: {e}"),
    }

    Ok(processes)
}

/// 批量杀进程树。
///
/// `pid_trees` 每个元素是一棵进程树的根 PID（整树由后端 taskkill /T 递归处理，
/// 故每个元素取首个根 PID 即可，其余 PID 用于统计 freed_bytes）。
///
/// 安全护栏：拒绝杀本应用自身进程；命中 cleaner_lock 锁定表的树整树跳过
/// （纵深防御，前端已过滤）。返回 `KillResult`，单棵树失败不中断其余，计入 failed。
#[tauri::command]
pub async fn kill_dev_processes<R: Runtime>(
    app: AppHandle<R>,
    pid_trees: Vec<Vec<u32>>,
) -> AppResult<KillResult> {
    if pid_trees.is_empty() {
        return Ok(KillResult {
            killed: 0,
            failed: 0,
            skipped_locked: 0,
            freed_bytes: 0,
        });
    }

    // 安全护栏：拒绝杀掉本应用自身进程
    let self_pid = sysinfo::get_current_pid().ok();
    let self_pid_u32 = self_pid.map(|p| p.as_u32());

    // 锁定表（不持 sysinfo 锁做 DB IO）；读失败降级为空集（与扫描降级策略一致）
    let locked_map = match db::pool(&app) {
        Ok(pool) => CleanerLockService::locked_map(&pool).await.unwrap_or_else(|e| {
            tracing::warn!("读取进程锁定表失败（本次不拦截锁定进程）: {e}");
            std::collections::HashMap::new()
        }),
        Err(e) => {
            tracing::warn!("获取数据库连接池失败（本次不拦截锁定进程）: {e}");
            std::collections::HashMap::new()
        }
    };

    // 持锁刷新 + 统计待杀树内存 + 标记锁定树（杀之前；锁定树不计内存）
    let mut freed_bytes = 0u64;
    let mut locked_roots: std::collections::HashSet<u32> = std::collections::HashSet::new();
    {
        let state = app.state::<AppState>();
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, dev_refresh_kind());
        for tree in &pid_trees {
            // 锁定校验：根 PID 指纹命中 → 整树跳过
            // （进程不在缓存中时按未锁定处理，维持原有行为）
            if let Some(&root) = tree.first() {
                if let Some(fp) = dev_scan::fingerprint_of_process(&system, root) {
                    if locked_map.contains_key(&fp) {
                        locked_roots.insert(root);
                        continue;
                    }
                }
            }
            for &pid_u32 in tree {
                if Some(pid_u32) == self_pid_u32 {
                    // 跳过自身，不计内存
                    continue;
                }
                if let Some(proc) = system.process(Pid::from_u32(pid_u32)) {
                    freed_bytes += proc.memory();
                }
            }
        }
    }

    // 并发杀树：各树独立无依赖，并行执行减少等待时间
    // 先统计自身进程被拒杀的树数（这些不提交并发任务）
    let self_rejected = pid_trees
        .iter()
        .filter(|tree| tree.first().map_or(false, |&r| Some(r) == self_pid_u32))
        .count() as u32;

    let futs: Vec<_> = pid_trees
        .iter()
        .filter_map(|tree| {
            let root = tree.first()?;
            if Some(*root) == self_pid_u32 || locked_roots.contains(root) {
                None
            } else {
                Some(kill_process_tree(*root))
            }
        })
        .collect();
    let results = futures::future::join_all(futs).await;
    let killed = results.iter().filter(|r| r.is_ok()).count() as u32;
    let failed = results.iter().filter(|r| r.is_err()).count() as u32 + self_rejected;

    Ok(KillResult {
        killed,
        failed,
        skipped_locked: locked_roots.len() as u32,
        freed_bytes,
    })
}

/// 批量修剪进程工作集（内存回收，不杀进程）。
///
/// 对每个 PID 调用 `SetProcessWorkingSetSizeEx(-1,-1,0)`，请求 OS 把进程
/// 不活跃的物理页换出到 pagefile，立即降低 RSS。进程继续运行。
///
/// 释放量统计：回收前读各进程 RSS → 逐个 trim → 刷新 sysinfo → 再读 RSS，
/// 差值之和即为 `freed_bytes`。
///
/// 适用场景：IDE（IDEA/VSCode）内存回收——这些进程不能杀（会丢工作），
/// 但其堆中有大量不活跃页可安全换出。
#[tauri::command]
pub async fn trim_dev_processes<R: Runtime>(
    app: AppHandle<R>,
    pids: Vec<u32>,
) -> AppResult<TrimResult> {
    if pids.is_empty() {
        return Ok(TrimResult {
            trimmed: 0,
            failed: 0,
            skipped_locked: 0,
            freed_bytes: 0,
        });
    }

    // 安全护栏：拒绝回收本应用自身进程（虽无害，但无意义）
    let self_pid_u32 = sysinfo::get_current_pid().ok().map(|p| p.as_u32());

    // 锁定表（不持 sysinfo 锁做 DB IO）；读失败降级为空集（与扫描降级策略一致）
    let locked_map = match db::pool(&app) {
        Ok(pool) => CleanerLockService::locked_map(&pool).await.unwrap_or_else(|e| {
            tracing::warn!("读取进程锁定表失败（本次不拦截锁定进程）: {e}");
            std::collections::HashMap::new()
        }),
        Err(e) => {
            tracing::warn!("获取数据库连接池失败（本次不拦截锁定进程）: {e}");
            std::collections::HashMap::new()
        }
    };

    let state = app.state::<AppState>();

    // 1. 回收前：读取各进程 RSS（持锁），命中锁定的 PID 直接跳过
    let mut skipped_locked = 0u32;
    let before_mem: std::collections::HashMap<u32, u64> = {
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, dev_refresh_kind());
        let mut mem = std::collections::HashMap::new();
        for &pid in &pids {
            if Some(pid) == self_pid_u32 {
                continue;
            }
            // 锁定校验（进程不在缓存中时按未锁定处理，维持原有行为）
            if let Some(fp) = dev_scan::fingerprint_of_process(&system, pid) {
                if locked_map.contains_key(&fp) {
                    skipped_locked += 1;
                    continue;
                }
            }
            if let Some(proc) = system.process(Pid::from_u32(pid)) {
                mem.insert(pid, proc.memory());
            }
        }
        mem
    };

    // 2. 逐个修剪工作集（OS 调用，不持 sysinfo 锁；锁定/自身 PID 已被排除）
    let mut trimmed = 0u32;
    let mut failed = 0u32;
    for &pid in before_mem.keys() {
        match trim_working_set(pid) {
            Ok(()) => trimmed += 1,
            Err(_) => failed += 1,
        }
    }

    // 3. 给 OS 一点时间完成换页后再读取（trim 是异步的页回收）
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;

    // 4. 回收后：刷新并再读 RSS（持锁），计算差值
    let freed_bytes = {
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, dev_refresh_kind());
        before_mem
            .iter()
            .filter_map(|(&pid, &before)| {
                let after = system
                    .process(Pid::from_u32(pid))
                    .map(|proc| proc.memory())
                    .unwrap_or(0);
                // RSS 下降量（上升或不变计为 0）
                before.checked_sub(after).filter(|&d| d > 0)
            })
            .sum::<u64>()
    };

    Ok(TrimResult {
        trimmed,
        failed,
        skipped_locked,
        freed_bytes,
    })
}

/// 查询系统物理内存概况（总量/已用/可用/使用率）。
///
/// 供清理抽屉顶部内存条展示。复用 AppState 的 sysinfo System，仅刷新内存。
#[tauri::command]
pub async fn get_system_memory<R: Runtime>(app: AppHandle<R>) -> AppResult<SystemMemory> {
    let state = app.state::<AppState>();
    let mut system = state.system().lock().expect("sysinfo mutex poisoned");

    // sysinfo 0.32：refresh_memory 刷新 RAM/Swap 使用
    system.refresh_memory();

    let total = system.total_memory();
    let used = system.used_memory();
    let available = total.saturating_sub(used);
    let used_percent = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    Ok(SystemMemory {
        total_bytes: total,
        used_bytes: used,
        available_bytes: available,
        used_percent,
    })
}

/// 进程锁定 CRUD（薄层，业务在 CleanerLockService）。
///
/// 锁定按进程指纹（cmdline 优先，cwd+name 兜底）持久化到 cleaner_lock 表，
/// 与 PID 解耦——进程重启后 PID 变化不影响锁定，条目永不自动删除。
/// 语义：指纹相同的多个进程实例会被一并锁定/解锁（指纹无法区分实例）。

/// 列出全部锁定条目（含未运行的进程，按锁定时间倒序）。
#[tauri::command]
pub async fn list_cleaner_locks<R: Runtime>(app: AppHandle<R>) -> AppResult<Vec<CleanerLock>> {
    let pool = db::pool(&app)?;
    CleanerLockService::list(&pool).await
}

/// 锁定一个进程（按指纹去重，重复锁定幂等）。返回该指纹的锁定条目。
#[tauri::command]
pub async fn add_cleaner_lock<R: Runtime>(
    app: AppHandle<R>,
    input: CleanerLockInput,
) -> AppResult<CleanerLock> {
    let pool = db::pool(&app)?;
    CleanerLockService::add(&pool, input).await
}

/// 解除单条进程锁定（lock_id 来自扫描结果 DevProcInfo.lock_id）。
#[tauri::command]
pub async fn remove_cleaner_lock<R: Runtime>(
    app: AppHandle<R>,
    lock_id: i64,
) -> AppResult<()> {
    let pool = db::pool(&app)?;
    CleanerLockService::remove(&pool, lock_id).await
}

/// 清空全部进程锁定条目（「解锁全部」）。
#[tauri::command]
pub async fn clear_cleaner_locks<R: Runtime>(app: AppHandle<R>) -> AppResult<()> {
    let pool = db::pool(&app)?;
    CleanerLockService::clear(&pool).await
}
