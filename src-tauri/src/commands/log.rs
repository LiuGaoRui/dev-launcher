//! log 命令薄层：subscribe_log / read_log_history / list_log_dates / clear_log
//!
//! 签名对齐 docs/03-命令清单.md §六。
//! 从 AppHandle 取 pool + AppState.logs_root，转调 logs 模块。
//!
//! Channel 参数（ADR-002）：`on_event: Channel<LogChunk>` 由 Tauri 自动注入，
//! 前端 invoke 时传 `onEvent: new Channel<LogChunk>()`。tail task 后台运行，
//! 命令本身 spawn 后即返回 Ok(())，不阻塞。

use std::fs::OpenOptions;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, Runtime};

use crate::db;
use crate::error::AppResult;
use crate::logs::paths;
use crate::logs::reader;
use crate::logs::tail;
use crate::logs::{LogChunk, LogHistoryPage};
use crate::services::ProjectService;
use crate::state::AppState;

/// 历史分页读取的单页字节上限（防止一次性把超大日志读进内存）。
const HISTORY_PAGE_LIMIT_MAX: u64 = 256 * 1024; // 256KB

/// 订阅实时日志：启动 tail task，新内容通过 on_event Channel 推送。
///
/// 命令立即返回；tail task 在后台运行，前端 GC Channel 时自动退出（ADR-002）。
/// 项目未运行也能订阅（看已落盘的当日日志，只是没有新增）。
#[tauri::command]
pub async fn subscribe_log<R: Runtime>(
    app: AppHandle<R>,
    project_id: i64,
    on_event: Channel<LogChunk>,
) -> AppResult<()> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, project_id).await?;
    let logs_root = app.state::<AppState>().logs_root().to_path_buf();

    // 当日日志路径（spawn 写日志用的是同一规则，见 spawn::log_file_path）
    let log_path = paths::date_log_path(&logs_root, &project.name, &paths::today());

    // detach：tail task 自行运行，命令立即返回
    tail::spawn_tail_task(log_path, on_event);
    Ok(())
}

/// 历史日志分页读取：从 `date`（YYYYMMDD）日志的 `offset` 处读 `limit` 字节。
///
/// `limit` 超过 256KB 会被截断，防止超大单页。
#[tauri::command]
pub async fn read_log_history<R: Runtime>(
    app: AppHandle<R>,
    project_id: i64,
    date: String,
    offset: u64,
    limit: u64,
) -> AppResult<LogHistoryPage> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, project_id).await?;
    let logs_root = app.state::<AppState>().logs_root().to_path_buf();

    let path = paths::date_log_path(&logs_root, &project.name, &date);
    let safe_limit = limit.min(HISTORY_PAGE_LIMIT_MAX);
    reader::read_history(&path, offset, safe_limit)
}

/// 列出某项目的全部历史日志日期（降序，最新在前）。
#[tauri::command]
pub async fn list_log_dates<R: Runtime>(
    app: AppHandle<R>,
    project_id: i64,
) -> AppResult<Vec<String>> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, project_id).await?;
    let logs_root = app.state::<AppState>().logs_root().to_path_buf();
    paths::list_log_dates(&logs_root, &project.name)
}

/// 清空日志：`date=None` 清当天，否则清指定日期。truncate 保留文件本身。
///
/// 若有 tail task 正在订阅，下次轮询检测到 file_size < offset 会自动重读全量，
/// 无需额外通知机制。
#[tauri::command]
pub async fn clear_log<R: Runtime>(
    app: AppHandle<R>,
    project_id: i64,
    date: Option<String>,
) -> AppResult<()> {
    let pool = db::pool(&app)?;
    let project = ProjectService::get(&pool, project_id).await?;
    let logs_root = app.state::<AppState>().logs_root().to_path_buf();

    let date = date.unwrap_or_else(paths::today);
    let path = paths::date_log_path(&logs_root, &project.name, &date);

    // 仅对已存在文件 truncate 清空内容（不创建新文件，避免 list_log_dates
    // 列出不存在的空日期）。文件不存在视为 no-op。
    if path.exists() {
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)?;
    }
    Ok(())
}
