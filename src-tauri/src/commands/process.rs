//! process 命令薄层：start_project / stop_project / restart_project
//!
//! 签名对齐 docs/03-命令清单.md §四。
//! 从 AppState 取 registry + logs_root，从 AppHandle 取 DB pool。

use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::now_iso;
use crate::process::job_object::JobHandle;
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
#[tauri::command]
pub async fn stop_project<R: Runtime>(app: AppHandle<R>, id: i64) -> AppResult<()> {
    let state = app.state::<AppState>();

    if !state.registry().contains(id) {
        return Err(AppError::NotRunning(id));
    }

    // 先调 job.terminate（持锁短暂，不 await）
    let terminate_ok = state
        .registry()
        .with_mut(id, |p| p.job.terminate())
        .unwrap_or(Ok(()));

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

    // 显式移除（drop child 触发 kill_on_drop，drop job 触发 CloseHandle → 已无进程）
    state.registry().remove(id);

    terminate_ok?;

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
