//! cleaner 命令薄层：scan_dev_processes / kill_dev_processes / get_system_memory。
//!
//! 内存清理器后端：扫描整机开发进程（java/node），智能分类 + 推荐清理目标，
//! 批量杀进程树，查询系统内存概况。
//!
//! 与 commands/process.rs 的区别：process 管理本工具**托管**的项目进程；
//! cleaner 面向全系统**任意**开发进程（含 IDE 派生、孤立残留），用于释放内存。

use serde::Serialize;
use sysinfo::{Pid, ProcessesToUpdate};
use tauri::{AppHandle, Manager, Runtime};

use crate::error::AppResult;
use crate::process::dev_scan::{self, DevProcInfo};
use crate::process::kill::kill_process_tree;
use crate::state::AppState;

/// 批量杀进程的结果。
#[derive(Debug, Clone, Serialize)]
pub struct KillResult {
    /// 成功杀死的进程树数量
    pub killed: u32,
    /// 杀失败的进程树数量
    pub failed: u32,
    /// 杀死前这些进程树的内存总和（字节，用于展示「已释放」）
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

    // 刷新全系统进程（cpu + 内存）后扫描
    let processes = {
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes(ProcessesToUpdate::All, true);
        dev_scan::scan_dev_processes(&system, self_pid)
    };

    Ok(processes)
}

/// 批量杀进程树。
///
/// `pid_trees` 每个元素是一棵进程树的根 PID（整树由后端 taskkill /T 递归处理，
/// 故每个元素取首个根 PID 即可，其余 PID 用于统计 freed_bytes）。
///
/// 返回 `KillResult { killed, failed, freed_bytes }`。
/// 单棵树失败不中断其余，计入 failed。
#[tauri::command]
pub async fn kill_dev_processes<R: Runtime>(
    app: AppHandle<R>,
    pid_trees: Vec<Vec<u32>>,
) -> AppResult<KillResult> {
    if pid_trees.is_empty() {
        return Ok(KillResult {
            killed: 0,
            failed: 0,
            freed_bytes: 0,
        });
    }

    // 安全护栏：拒绝杀掉本应用自身进程
    let self_pid = sysinfo::get_current_pid().ok();
    let self_pid_u32 = self_pid.map(|p| p.as_u32());

    // 先在持锁期间统计待杀进程树的内存（杀之前）
    let mut freed_bytes = 0u64;
    {
        let state = app.state::<AppState>();
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes(ProcessesToUpdate::All, true);
        for tree in &pid_trees {
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
            if Some(*root) == self_pid_u32 {
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
