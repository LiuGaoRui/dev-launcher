//! process 命令薄层：start_project / stop_project / restart_project / probe_statuses
//!
//! 进程**脱离**管理器生命周期：spawn 不挂 Job Object、不设 kill_on_drop，
//! 关闭软件后子进程继续运行。停止用 `taskkill /F /T` 杀整树；
//! 重开软件后 probe_statuses 通过 DB last_pid + sysinfo 验活接管展示。
//!
//! 签名对齐 docs/03-命令清单.md §四。
//! 从 AppState 取 registry + logs_root + system，从 AppHandle 取 DB pool。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::Row;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Runtime};

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::now_iso;
use crate::process::build::{run_build, BuildEvent, BuildResult};
use crate::process::monitor::{collect_tcp_sockets, probe_one, ProjectStatus};
use crate::process::registry::{ProcessSnapshot, RunningProcess};
use crate::process::spawn::{log_file_path, spawn_command};
use crate::process::StartResult;
use crate::services::ProjectService;
use crate::state::AppState;

/// 后台 wait 任务轮询 try_wait 的间隔（ms）。
const WAIT_CLEANUP_INTERVAL_MS: u64 = 500;

/// 启动项目。
///
/// 流程：取 project → 校验未运行 → 算日志路径 → spawn → 入 registry
/// → 更新 DB last_pid/last_start_time → spawn 后台 wait task（退出时清理）。
///
/// 进程脱离管理器生命周期：不挂 Job Object、不设 kill_on_drop，
/// 关闭软件后子进程继续运行。停止由 stop_project 的 taskkill /F /T 完成。
#[tauri::command]
pub async fn start_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<StartResult> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, id).await?;
    let state = app.state::<AppState>();

    // 校验未运行（registry 内本会话进程，或 DB last_pid 指向的外部进程仍存活）
    if state.registry().contains(id) {
        return Err(AppError::AlreadyRunning(id));
    }
    if let Some(last_pid) = project.last_pid {
        let system = state.system().lock().expect("sysinfo mutex poisoned");
        if pid_alive(last_pid as u32, &system) {
            return Err(AppError::AlreadyRunning(id));
        }
    }

    // 计算日志路径
    let log_path = log_file_path(state.logs_root(), &project.name)?;
    let started_at = now_iso();

    // spawn 进程
    let (child, pid) = spawn_command(&project, &log_path)?;

    // 入 registry（已二次校验 contains，但 insert 仍会校验冲突）
    let proc = RunningProcess {
        child,
        pid,
        log_path: log_path.clone(),
        started_at: started_at.clone(),
    };
    state.registry().insert(id, proc)?;

    // 更新 DB 缓存
    sqlx::query(
        "UPDATE project SET last_pid = ?, last_start_time = ?, update_time = datetime('now') WHERE id = ?",
    )
    .bind(pid as i64)
    .bind(&started_at)
    .bind(id)
    .execute(&pool)
    .await?;

    // spawn 后台 wait task：进程退出后清理 registry + 更新 last_stop_time
    let app2 = app.clone();
    tokio::spawn(async move {
        wait_and_cleanup(&app2, id).await;
    });

    Ok(StartResult {
        root_pid: pid,
        log_path: log_path.to_string_lossy().into_owned(),
        started_at,
    })
}

/// 停止项目：用 `taskkill /F /T /PID` 杀整树，兼顾本会话进程与重开接管的外部进程。
///
/// pid 来源：registry 有该项目 → 用 registry 的 pid；否则查 DB 的 last_pid。
/// 两者皆无 → `NotRunning`。
///
/// 杀树后：registry 有该项目则移除（drop child，因无 kill_on_drop，安全无副作用），
/// 并更新 DB last_stop_time + 清空 last_pid。
#[tauri::command]
pub async fn stop_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<()> {
    let state = app.state::<AppState>();

    // 取 pid：registry 优先（本会话进程），否则查 DB（重开接管的外部进程）
    let pid: u32 = if let Some(snap) = state.registry().snapshot(id) {
        snap.pid
    } else {
        let pool = db::pool(&app)?;
        let last_pid: Option<i64> = sqlx::query_scalar(
            "SELECT last_pid FROM project WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&pool)
        .await?
        .flatten();
        match last_pid {
            Some(p) => p as u32,
            None => return Err(AppError::NotRunning(id)),
        }
    };

    // taskkill /F /T 杀整树
    kill_process_tree(pid).await?;

    // registry 有该项目则移除（drop child，无 kill_on_drop 不影响子进程）
    state.registry().remove(id);

    // 更新 DB last_stop_time + 清空 last_pid
    let pool = db::pool(&app)?;
    clear_runtime_after_stop(&pool, id).await?;

    Ok(())
}

/// 重启：stop → start。必须确认旧进程完全退出后再启动，否则端口占用。
#[tauri::command]
pub async fn restart_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<StartResult> {
    // 若正在运行则先停止（stop 内部已校验 NotRunning，这里容忍未运行直接 start）
    if app.state::<AppState>().registry().contains(id) {
        stop_project(app.clone(), id).await?;
    }
    start_project(app, id).await
}

/// 构建项目：执行 build_cmd，stdout/stderr 实时推 Channel，跑完返回退出码与耗时。
///
/// 与 start_project 的区别：一次性进程（不入 registry、不用 Job Object），
/// stdout/stderr 走 piped 实时推前端（不写日志文件），由 BuildResult 返回退出码。
///
/// 命令会 await 到构建结束才返回；构建中前端关闭 Channel → 读取 task 退出 →
/// kill_on_drop 兜底回收子进程。
#[tauri::command]
pub async fn build_project<R: Runtime>(
    app: AppHandle<R>,
    id: i64,
    on_event: Channel<BuildEvent>,
) -> AppResult<BuildResult> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, id).await?;
    run_build(&project, on_event).await
}

/// 探测项目运行态（CPU/内存/端口），阶段 5 监控面板用。
///
/// 候选集 = registry 内本会话进程 ∪ { DB 中 last_pid 非空的项目 }，
/// 后者用于「重开软件后接管外部进程」展示。验活用 sysinfo，进程已退出则清理残留缓存。
///
/// - `ids = None`/空：探测上述候选集全部
/// - `ids = Some([...])`：仅探测给定项目（前端可传当前可见集减少开销）
///
/// 返回的 Vec 仅含「确实运行中」的项目；前端对不在结果中的 id 视为 stopped。
///
/// 流程：refresh 系统进程（cpu 基线）→ 取 TCP socket 全表一次 →
/// 逐项目 probe（聚合进程树 CPU/内存 + 端口归属校验）。
#[tauri::command]
pub async fn probe_statuses<R: Runtime>(
    app: AppHandle<R>,
    ids: Option<Vec<i64>>,
) -> AppResult<Vec<ProjectStatus>> {
    let state = app.state::<AppState>();
    let registry = state.registry();
    let pool = db::pool(&app)?;

    // 候选集 + DB 运行时信息
    let (mut candidates, runtime_map) = match ids {
        Some(v) if !v.is_empty() => {
            // 定向探测：仅对不在 registry 中的候选查 DB
            let db_only: Vec<i64> = v
                .iter()
                .filter(|id| !registry.contains(**id))
                .copied()
                .collect();
            let runtime_map = fetch_runtime_info(&pool, &db_only, state.logs_root()).await?;
            (v, runtime_map)
        }
        _ => {
            // 全量扫描：registry 内项目 + DB 全部 last_pid 非空的项目
            let mut candidates = registry.running_ids();
            let runtime_map = fetch_runtime_info(&pool, &[], state.logs_root()).await?;
            for id in runtime_map.keys() {
                if !candidates.contains(id) {
                    candidates.push(*id);
                }
            }
            (candidates, runtime_map)
        }
    };
    candidates.sort_unstable();

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // 批量取 expected_ports（一次 SQL）
    let ports_map = fetch_expected_ports(&pool, &candidates).await?;

    // TCP 全表一次查（不依赖 system，放在锁外减少临界区）
    let sockets = collect_tcp_sockets();

    // 刷新系统进程（cpu 基线）并逐项目 probe
    let mut results = Vec::with_capacity(candidates.len());
    let mut dead_ids: Vec<i64> = Vec::new();
    {
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes(ProcessesToUpdate::All, true);
        for id in &candidates {
            // registry 有（本会话进程）→ 直接用；否则查 runtime_map（外部接管进程）
            let snap = if let Some(s) = registry.snapshot(*id) {
                s
            } else if let Some(r) = runtime_map.get(id) {
                if !pid_alive(r.pid, &system) {
                    dead_ids.push(*id);
                    continue;
                }
                r.clone()
            } else {
                continue;
            };
            let ports = ports_map.get(id).cloned().unwrap_or_default();
            results.push(probe_one(*id, &snap, &ports, &system, &sockets));
        }
    }

    // 批量清理已退出外部进程的残留缓存（不持 system 锁）
    if !dead_ids.is_empty() {
        let _ = clear_runtime_after_stop_batch(&pool, &dead_ids).await;
    }

    Ok(results)
}

// ===== 辅助 =====

/// 停止后清理 DB 运行时缓存：清空 last_pid + 写 last_stop_time。
async fn clear_runtime_after_stop(pool: &sqlx::Pool<sqlx::Sqlite>, id: i64) -> AppResult<()> {
    sqlx::query(
        "UPDATE project SET last_pid = NULL, last_stop_time = ?, update_time = datetime('now') WHERE id = ?",
    )
    .bind(now_iso())
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 批量清理 DB 运行时缓存（多条项目一次性 UPDATE，避免 N 次 DB 往返）。
async fn clear_runtime_after_stop_batch(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    ids: &[i64],
) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "UPDATE project SET last_pid = NULL, last_stop_time = ?, update_time = datetime('now') WHERE id IN ({})",
        in_placeholders(ids.len())
    );
    let mut q = sqlx::query(&sql).bind(now_iso());
    for id in ids {
        q = q.bind(id);
    }
    q.execute(pool).await?;
    Ok(())
}

/// 后台轮询某 project 的 child 是否退出，退出后清理 registry + 更新 DB。
///
/// 仅用于本会话 spawn 的进程（registry 内有 child）。外部接管进程无 child、无 wait task，
/// 其退出由 probe_statuses 的 sysinfo 验活兜底清理。
///
/// 不能持 registry 锁 await，故用 try_wait 轮询而非 child.wait().await。
async fn wait_and_cleanup<R: Runtime>(app: &AppHandle<R>, project_id: i64) {
    let state = app.state::<AppState>();
    loop {
        let exited = state.registry().with_mut(project_id, |p| {
            match p.child.try_wait() {
                Ok(Some(_status)) => true,
                Ok(None) => false,
                Err(_) => true, // 取不到状态视为退出
            }
        });
        match exited {
            Some(true) => break,
            Some(false) => {
                tokio::time::sleep(Duration::from_millis(WAIT_CLEANUP_INTERVAL_MS)).await;
            }
            None => return, // registry 中已无此项目（被 stop 移除），退出
        }
    }

    // 进程已退出：移除 registry（幂等），清理 DB 缓存（清空 last_pid + 写 last_stop_time）
    let _ = state.registry().remove(project_id);
    if let Ok(pool) = db::pool(app) {
        let _ = clear_runtime_after_stop(&pool, project_id).await;
    }
}

/// 构建 SQL `IN (...)` 占位符字符串，`n` 个 `?` 以逗号分隔。
fn in_placeholders(n: usize) -> String {
    (0..n).map(|_| "?").collect::<Vec<_>>().join(",")
}

/// 批量取多个项目的 expected_ports，避免 probe 时 N+1 查询。
///
/// 一次 SQL `SELECT id, expected_ports FROM project WHERE id IN (...)`，
/// expected_ports 为 JSON TEXT，手动反序列化为 `Vec<String>`。
async fn fetch_expected_ports(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    ids: &[i64],
) -> AppResult<HashMap<i64, Vec<String>>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let sql = format!(
        "SELECT id, expected_ports FROM project WHERE id IN ({})",
        in_placeholders(ids.len())
    );
    let mut q = sqlx::query(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let rows = q.fetch_all(pool).await?;

    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let ports_json: String = row.try_get("expected_ports")?;
        let ports: Vec<String> = serde_json::from_str(&ports_json).unwrap_or_default();
        map.insert(id, ports);
    }
    Ok(map)
}

/// 批量取项目的运行时信息（last_pid / last_start_time / name→log_path），
/// 返回 `ProcessSnapshot` 供 probe 循环直接使用。
///
/// - `ids` 非空：查 `WHERE id IN (...)` 的指定项目
/// - `ids` 为空：查全部 `WHERE last_pid IS NOT NULL`（全量扫描模式）
async fn fetch_runtime_info(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    ids: &[i64],
    logs_root: &Path,
) -> AppResult<HashMap<i64, ProcessSnapshot>> {
    let rows: Vec<_> = if ids.is_empty() {
        // 空 = 全量扫描：查全部 last_pid 非空的项目
        sqlx::query(
            "SELECT id, name, last_pid, last_start_time FROM project WHERE last_pid IS NOT NULL",
        )
        .fetch_all(pool)
        .await?
    } else {
        let sql = format!(
            "SELECT id, name, last_pid, last_start_time FROM project WHERE id IN ({})",
            in_placeholders(ids.len())
        );
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(id);
        }
        q.fetch_all(pool).await?
    };

    let mut map = HashMap::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let last_pid: Option<i64> = row.try_get("last_pid")?;
        let last_start_time: Option<String> = row.try_get("last_start_time")?;
        if let Some(pid) = last_pid {
            let log_path = crate::process::spawn::log_file_path(logs_root, &name)
                .unwrap_or_else(|_| PathBuf::new());
            map.insert(
                id,
                ProcessSnapshot {
                    pid: pid as u32,
                    log_path,
                    started_at: last_start_time.unwrap_or_default(),
                },
            );
        }
    }
    Ok(map)
}

/// 用 sysinfo 判断 PID 是否存活。
fn pid_alive(pid: u32, system: &System) -> bool {
    system.process(Pid::from_u32(pid)).is_some()
}

/// 用 `taskkill /F /T /PID <pid>` 杀掉指定进程及其全部后代。
///
/// `/F` 强制终止，`/T` 递归杀子进程树。Windows 自带命令。
#[cfg(windows)]
async fn kill_process_tree(pid: u32) -> AppResult<()> {
    use crate::process::spawn::CREATE_NO_WINDOW;

    let output = tokio::process::Command::new("taskkill")
        .args(["/F", "/T", "/PID"])
        .arg(pid.to_string())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .map_err(|e| AppError::Process(format!("调用 taskkill 失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Process(format!(
            "taskkill 终止进程 {pid} 失败: {}",
            stderr.trim()
        )));
    }
    Ok(())
}
