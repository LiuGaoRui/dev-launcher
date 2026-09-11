//! log 命令薄层：subscribe_log / unsubscribe_log / clear_log
//!
//! 日志按类型分 start（启动日志）/ build（构建日志），均为实时 tail。
//! 每次启动/构建前由 process 模块 truncate，日志文件只含本次输出。
//!
//! 订阅生命周期（ADR-006）：`subscribe_log` 返回后端分配的 `sub_id`，前端切换
//! 订阅时显式调 `unsubscribe_log(sub_id)` 取消，不再依赖 Channel 的 GC 时机。
//! tail task 退出时会自清理登记项，重复取消是安全的 no-op。
//!
//! Channel 参数：`on_event: Channel<LogChunk>` 由 Tauri 自动注入，
//! 前端 invoke 时传 `onEvent: new Channel<LogChunk>()`。

use std::fs::OpenOptions;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Runtime};

use crate::db;
use crate::error::AppResult;
use crate::logs::paths::{log_path_of, LogType};
use crate::logs::tail;
use crate::logs::LogChunk;
use crate::services::ProjectService;
use crate::state::AppState;

/// 订阅实时日志：登记订阅 + 启动 tail task，返回订阅 id。
///
/// 项目未运行/未构建也能订阅（看已落盘的日志，只是没有新增）。
/// 前端须保存返回的 `sub_id`，切换订阅或离开页面时调 [`unsubscribe_log`]。
///
/// `log_type`："start"（启动日志）或 "build"（构建日志），非法值兜底为 "start"。
#[tauri::command]
pub async fn subscribe_log<R: Runtime>(
    app: AppHandle<R>,
    project_id: i64,
    log_type: String,
    on_event: Channel<LogChunk>,
) -> AppResult<u64> {
    let pool = db::pool(&app)?;
    // 校验项目存在：不存在直接报错，而不是静默 tail 一个不存在的路径
    ProjectService::get(&pool, project_id).await?;

    let state = app.state::<AppState>();
    let log_path = log_path_of(state.logs_root(), project_id, LogType::parse(&log_type));

    // 先登记再 spawn：若 task 先自然退出并调 finish，后 insert 会留下残留项
    let registry = state.log_subs();
    let (sub_id, cancel) = registry.register();
    tail::spawn_tail_task(log_path, sub_id, cancel, registry, on_event);
    Ok(sub_id)
}

/// 取消日志订阅：置取消标志，tail task 在下一个轮询节点退出。
///
/// 幂等：订阅 id 不存在（已取消 / task 已自行退出）时返回 Ok。
#[tauri::command]
pub async fn unsubscribe_log<R: Runtime>(
    app: AppHandle<R>,
    subscription_id: u64,
) -> AppResult<()> {
    app.state::<AppState>().log_subs().cancel(subscription_id);
    Ok(())
}

/// 清空日志：truncate 指定类型的日志文件，保留文件本身。
///
/// 若有 tail task 正在订阅，下次轮询检测到 file_size < offset 会自动重读全量，
/// 无需额外通知机制。
///
/// `log_type`："start" / "build"，非法值兜底为 "start"。
#[tauri::command]
pub async fn clear_log<R: Runtime>(
    app: AppHandle<R>,
    project_id: i64,
    log_type: String,
) -> AppResult<()> {
    let pool = db::pool(&app)?;
    ProjectService::get(&pool, project_id).await?;

    let state = app.state::<AppState>();
    let path = log_path_of(state.logs_root(), project_id, LogType::parse(&log_type));

    // 仅对已存在文件 truncate 清空内容（不创建新文件）。
    if path.exists() {
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)?;
    }
    Ok(())
}
