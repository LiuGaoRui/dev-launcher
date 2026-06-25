//! 实时日志 tail task
//!
//! 启动一个 tokio 后台任务，轮询读取日志文件的增量内容，通过
//! `tauri::ipc::Channel<LogChunk>` 推送给前端（ADR-002）。
//!
//! 生命周期（ADR-002 自动退订）：
//!   前端不再持有 Channel 引用 → GC → `on_event.send()` 返回 Err → task break 退出。
//!   无需显式 unsubscribe 命令。
//!
//! 起点策略（已确认）：从文件开头推送今日全部已有内容，再实时追加新行。
//!   简单可靠，能完整看到启动日志。
//!
//! 截断/轮转处理：若 `file_size < last_offset`（被 clear_log truncate 或外部清空），
//!   重置 offset=0 重读全量，避免一直 seek 到失效位置读不到数据。

use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;

use tauri::ipc::Channel;
use tracing::{debug, warn};

use crate::error::AppResult;
use crate::logs::LogChunk;

/// tail 轮询间隔（ms）。固定轮询，不引 notify 依赖（MVP 足够）。
const POLL_INTERVAL_MS: u64 = 250;
/// 日志文件尚未落盘时的重试上限（项目刚 spawn，文件可能还没创建）。
/// 超过后视为该次订阅无日志，task 退出（前端重新订阅即可）。
const OPEN_RETRY_INTERVAL_MS: u64 = 500;
const OPEN_RETRY_MAX: u32 = 20; // 20 × 500ms = 10s

/// 启动 tail 后台任务。命令层 spawn 它后立即返回 Ok(())，不阻塞调用者。
///
/// 返回 `JoinHandle` 供调用方 detach（不 await）。
pub fn spawn_tail_task(log_path: PathBuf, on_event: Channel<LogChunk>) {
    tokio::spawn(async move {
        if let Err(e) = run_tail(&log_path, &on_event).await {
            warn!("tail task for {} 异常退出: {e}", log_path.display());
        } else {
            debug!("tail task for {} 正常退出", log_path.display());
        }
    });
}

async fn run_tail(log_path: &PathBuf, on_event: &Channel<LogChunk>) -> AppResult<()> {
    // 1. 等待文件就绪（项目刚 spawn 时文件可能还没创建）
    let mut file = {
        let mut retries = 0u32;
        loop {
            match std::fs::File::open(log_path) {
                Ok(f) => break f,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    if retries >= OPEN_RETRY_MAX {
                        // 文件迟迟不出现，视为无日志，安静退出
                        debug!("tail: 日志文件 {} 未出现，退出", log_path.display());
                        return Ok(());
                    }
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(OPEN_RETRY_INTERVAL_MS)).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
    };

    // 2. 推送今日全部已有内容（起点策略：从 offset 0）
    let mut offset = 0u64;
    offset = push_all_available(&mut file, offset, on_event).await?;

    // 3. 增量轮询循环
    loop {
        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

        let size = match std::fs::metadata(log_path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // 文件被删了，等它重新出现
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        if size < offset {
            // 文件被截断/轮转（clear_log 或外部清空）→ 重置到 0 重读全量
            debug!(
                "tail: 检测到文件截断 ({} < {})，重置 offset=0",
                size, offset
            );
            offset = 0;
            file.seek(SeekFrom::Start(0))?;
        }

        if size > offset {
            offset = push_all_available(&mut file, offset, on_event).await?;
        }
    }
}

/// 从当前 offset 读到文件末尾的全部新增内容，作为一个 LogChunk 推送。
/// 返回新的 offset（= 文件当前末尾）。send 失败（前端 GC）→ 返回 Err 中断循环。
async fn push_all_available(
    file: &mut std::fs::File,
    offset: u64,
    on_event: &Channel<LogChunk>,
) -> AppResult<u64> {
    file.seek(SeekFrom::Start(offset))?;

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let new_offset = offset + buf.len() as u64;

    if !buf.is_empty() {
        let text = String::from_utf8_lossy(&buf).into_owned();
        // Channel GC 时 send 返回 Err → 这里透传，run_tail 循环退出
        on_event
            .send(LogChunk {
                offset,
                text,
            })
            .map_err(|e| crate::error::AppError::Process(format!("日志推送失败(Channel 已关闭): {e}")))?;
    }

    Ok(new_offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// 模拟 tail 的增量读取逻辑（不经过 Channel，纯文件 IO 验证）。
    /// Channel 的 GC 自动退订行为依赖真实 Tauri IPC，由手动集成测试覆盖。
    fn write_log(path: &std::path::Path, content: &str) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn read_incremental_via_seek() {
        // 模拟 tail 的增量读取逻辑（不经过 Channel，纯文件 IO 验证）
        let tmp = tempfile::tempdir().unwrap();
        let log = tmp.path().join("a.log");

        write_log(&log, "line1\n");
        let mut file = std::fs::File::open(&log).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        assert_eq!(String::from_utf8_lossy(&buf), "line1\n");
        let mut offset = buf.len() as u64;

        // 追加新内容
        write_log(&log, "line2\nline3\n");
        file.seek(SeekFrom::Start(offset)).unwrap();
        buf.clear();
        file.read_to_end(&mut buf).unwrap();
        assert_eq!(String::from_utf8_lossy(&buf), "line2\nline3\n");
        offset += buf.len() as u64;

        // 截断后重读
        std::fs::write(&log, "reset\n").unwrap();
        let size = std::fs::metadata(&log).unwrap().len();
        assert!(size < offset);
        file.seek(SeekFrom::Start(0)).unwrap();
        buf.clear();
        file.read_to_end(&mut buf).unwrap();
        assert_eq!(String::from_utf8_lossy(&buf), "reset\n");
    }
}
