//! 日志模块：实时 tail + 历史分页读取
//!
//! 模块结构（见 docs/10-目录结构规范.md §三）：
//! - `paths`：日志路径计算（复用 spawn 的 sanitize_name，保证读写同路径）
//! - `reader`：历史日志分页读取（seek + 读 limit 字节）
//! - `tail`：tokio tail task，轮询日志文件增量，通过 ipc::Channel 推送（ADR-002）
//!
//! 数据流（ADR-001 + ADR-002）：
//!   子进程 stdout/stderr → append 落盘 {logs_root}/{name}/{YYYYMMDD}.log
//!   → tail task 轮询读增量 → Channel<LogChunk> → 前端日志面板
//!   前端 Channel GC → send() 失败 → tail task 自动退出（无需显式退订）

pub mod paths;
pub mod reader;
pub mod tail;

use serde::Serialize;

/// 实时日志推送的数据块（对齐 docs/03-命令清单.md §六 LogChunk）。
///
/// - `offset`：本块数据在日志文件中的字节偏移
/// - `text`：新增的文本内容（可能含多行）
#[derive(Debug, Clone, Serialize)]
pub struct LogChunk {
    pub offset: u64,
    pub text: String,
}

/// 历史日志分页读取结果（对齐 docs/03-命令清单.md §六 LogHistoryPage）。
///
/// - `total_size`：日志文件总字节数
/// - `data`：本次读取到的文本（最多 limit 字节）
/// - `next_offset`：下一页起始偏移；None 表示已到文件末尾
#[derive(Debug, Clone, Serialize)]
pub struct LogHistoryPage {
    pub total_size: u64,
    pub data: String,
    pub next_offset: Option<u64>,
}
