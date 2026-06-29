//! log 命令薄层：subscribe_log / clear_log
//!
//! 日志按类型分 start（启动日志）/ build（构建日志），均为实时 tail。
//! 历史模式已移除（每次启动/构建前由 process 模块 truncate，日志文件只含本次输出）。
//!
//! Channel 参数（ADR-002）：`on_event: Channel<LogChunk>` 由 Tauri 自动注入，
//! 前端 invoke 时传 `onEvent: new Channel<LogChunk>()`。tail task 后台运行，
//! 命令本身 spawn 后即返回 Ok(())，不阻塞。

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

/// 订阅实时日志：启动 tail task，新内容通过 on_event Channel 推送。
///
/// 命令立即返回；tail task 在后台运行，前端 GC Channel 时自动退出（ADR-002）。
/// 项目未运行/未构建也能订阅（看已落盘的日志，只是没有新增）。
///
/// `log_type`："start"（启动日志）或 "build"（构建日志），非法值兜底为 "start"。
#[tauri::command]
pub async fn subscribe_log<R: Runtime>(
    app: AppHandle<R>,
    project_id: i64,
    log_type: String,
    on_event: Channel<LogChunk>,
) -> AppResult<()> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, project_id).await?;
    let logs_root = app.state::<AppState>().logs_root().to_path_buf();

    // 按类型解析日志路径（start.log / build.log）
    let log_path = log_path_of(&logs_root, &project.name, LogType::parse(&log_type));

    // detach：tail task 自行运行，命令立即返回
    tail::spawn_tail_task(log_path, on_event);
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
    let project = ProjectService::get(&pool, project_id).await?;
    let logs_root = app.state::<AppState>().logs_root().to_path_buf();

    let path = log_path_of(&logs_root, &project.name, LogType::parse(&log_type));

    // 仅对已存在文件 truncate 清空内容（不创建新文件）。
    if path.exists() {
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)?;
    }
    Ok(())
}
