//! process 命令薄层：start_project / stop_project / restart_project / probe_statuses
//!
//! 签名对齐 docs/03-命令清单.md §四。
//! 从 AppState 取 registry + logs_root + system，从 AppHandle 取 DB pool。

use std::collections::HashMap;
use std::time::Duration;

use sqlx::Row;
use sysinfo::ProcessesToUpdate;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Runtime};

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::now_iso;
use crate::process::build::{run_build, BuildEvent, BuildResult};
use crate::process::job_object::JobHandle;
use crate::process::monitor::{collect_tcp_sockets, probe_one, ProjectStatus};
use crate::process::registry::RunningProcess;
use crate::process::spawn::{log_file_path, spawn_command};
use crate::process::StartResult;
use crate::services::ProjectService;
use crate::state::AppState;

/// Job terminate 后轮询 ActiveProcesses 归零的超时（ms）。
const TERMINATE_POLL_TIMEOUT_MS: u64 = 5000;
/// Job terminate 后轮询 ActiveProcesses 的间隔（ms）。
const TERMINATE_POLL_INTERVAL_MS: u64 = 100;
/// 后台 wait 任务轮询 try_wait 的间隔（ms）。
const WAIT_CLEANUP_INTERVAL_MS: u64 = 500;

/// 启动项目。
///
/// 流程：取 project → 校验未运行 → 算日志路径 → spawn → 建 Job → assign → 入 registry
/// → 更新 DB last_pid/last_start_time → spawn 后台 wait task（退出时清理）。
#[tauri::command]
pub async fn start_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<StartResult> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, id).await?;
    let state = app.state::<AppState>();

    // 校验未运行
    if state.registry().contains(id) {
        return Err(AppError::AlreadyRunning(id));
    }

    // 计算日志路径
    let log_path = log_file_path(state.logs_root(), &project.name)?;
    let started_at = now_iso();

    // spawn 进程
    let (child, pid) = spawn_command(&project, &log_path)?;
    let raw_handle = child
        .raw_handle()
        .ok_or_else(|| AppError::Process("spawn 后无法获取进程句柄".into()))?;

    // 建 Job 并 assign（spawn-then-assign，竞态由 KILL_ON_JOB_CLOSE 兜底）
    let job = JobHandle::new()?;
    job.assign(raw_handle)?;

    // 入 registry（已二次校验 contains，但 insert 仍会校验冲突）
    let proc = RunningProcess {
        child,
        job,
        pid,
        log_path: log_path.clone(),
        started_at: started_at.clone(),
    };
    state.registry().insert(id, proc)?;

    // 更新 DB 缓存
    update_runtime_cache(&pool, id, Some(pid as i64), Some(&started_at), None).await?;

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

/// 停止项目：Job 优先杀整树 → 轮询 ActiveProcesses 归零（最多 5s）→ 兜底 kill child → 清理。
///
/// 若 job.terminate 失败，提前返回错误且**不从 registry 移除**（保留条目供重试/诊断），
/// 避免进程变孤儿无人管理。terminate 成功才进入清理流程。
#[tauri::command]
pub async fn stop_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<()> {
    let state = app.state::<AppState>();

    if !state.registry().contains(id) {
        return Err(AppError::NotRunning(id));
    }

    // 先调 job.terminate（持锁短暂，不 await）。失败则保留 registry 条目，提前返回错误。
    let terminate_ok = state
        .registry()
        .with_mut(id, |p| p.job.terminate())
        .unwrap_or(Ok(()));
    terminate_ok?;

    // 轮询 ActiveProcesses 归零
    let mut waited = 0u64;
    loop {
        let active = state
            .registry()
            .with_mut(id, |p| p.job.active_processes())
            .unwrap_or(Ok(0))
            .unwrap_or(0);
        if active == 0 {
            break;
        }
        if waited >= TERMINATE_POLL_TIMEOUT_MS {
            // 超时兜底：child.kill()
            state.registry().with_mut(id, |p| {
                let _ = p.child.start_kill();
            });
            break;
        }
        tokio::time::sleep(Duration::from_millis(TERMINATE_POLL_INTERVAL_MS)).await;
        waited += TERMINATE_POLL_INTERVAL_MS;
    }

    // terminate 已成功：显式移除（drop child 触发 kill_on_drop，drop job 触发 CloseHandle → 已无进程）
    state.registry().remove(id);

    // 更新 DB last_stop_time
    let pool = db::pool(&app)?;
    update_runtime_cache(&pool, id, None, None, Some(&now_iso())).await?;

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
/// - `ids = None`/空：探测 registry 中全部运行中项目
/// - `ids = Some([...])`：仅探测给定项目（前端可传当前可见集减少开销）
///
/// 返回的 Vec 仅含「运行中」的项目；前端对不在结果中的 id 视为 stopped。
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

    // 解析要探测的 id 列表：去重；过滤掉不在 registry 的（已停止）
    let mut to_probe: Vec<i64> = match ids {
        Some(v) if !v.is_empty() => v.into_iter().collect(),
        _ => registry.running_ids(),
    };
    to_probe.sort_unstable();
    to_probe.dedup();
    to_probe.retain(|id| registry.contains(*id));

    if to_probe.is_empty() {
        return Ok(Vec::new());
    }

    // 批量取这些项目的 expected_ports（一次 SQL，避免 N+1）
    let pool = db::pool(&app)?;
    let ports_map = fetch_expected_ports(&pool, &to_probe).await?;

    // TCP 全表一次查（不依赖 system，放在锁外减少临界区）
    let sockets = collect_tcp_sockets();

    // 刷新系统进程（cpu 基线）并逐项目 probe
    let mut results = Vec::with_capacity(to_probe.len());
    {
        // sysinfo cpu_usage 需两次 refresh 间隔才准确，持久化 System 复用基线
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes(ProcessesToUpdate::All, true);
        for id in &to_probe {
            if let Some(snap) = registry.snapshot(*id) {
                let ports = ports_map.get(id).cloned().unwrap_or_default();
                results.push(probe_one(*id, &snap, &ports, &system, &sockets));
            }
        }
    }

    Ok(results)
}

// ===== 辅助 =====

/// 更新 project 表的运行时缓存字段。
/// `pid` / `start` / `stop` 任一为 None 表示不更新该字段。
async fn update_runtime_cache(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    id: i64,
    pid: Option<i64>,
    start: Option<&str>,
    stop: Option<&str>,
) -> AppResult<()> {
    let mut sets: Vec<&str> = Vec::new();
    if pid.is_some() {
        sets.push("last_pid = ?");
    }
    if start.is_some() {
        sets.push("last_start_time = ?");
    }
    if stop.is_some() {
        sets.push("last_stop_time = ?");
    }
    if sets.is_empty() {
        return Ok(());
    }
    sets.push("update_time = datetime('now')");
    let sql = format!("UPDATE project SET {} WHERE id = ?", sets.join(", "));

    let mut q = sqlx::query(&sql);
    if let Some(p) = pid {
        q = q.bind(p);
    }
    if let Some(s) = start {
        q = q.bind(s);
    }
    if let Some(s) = stop {
        q = q.bind(s);
    }
    q.bind(id).execute(pool).await?;
    Ok(())
}

/// 后台轮询某 project 的 child 是否退出，退出后清理 registry + 更新 DB。
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

    // 进程已退出：移除 registry（幂等），更新 DB
    let _ = state.registry().remove(project_id);
    if let Ok(pool) = db::pool(app) {
        let _ = update_runtime_cache(&pool, project_id, None, None, Some(&now_iso())).await;
    }
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
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT id, expected_ports FROM project WHERE id IN ({placeholders})"
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

