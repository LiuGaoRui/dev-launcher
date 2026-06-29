//! 日志模块：实时 tail（启动日志 + 构建日志）
//!
//! 模块结构：
//! - `paths`：日志路径计算（按类型分 start.log / build.log，复用 spawn 的 sanitize_name）
//! - `tail`：tokio tail task，轮询日志文件增量，通过 ipc::Channel 推送（ADR-002）
//!
//! 数据流：
//!   子进程 stdout/stderr → append 落盘 {logs_root}/{name}/{start|build}.log
//!   → tail task 轮询读增量 → Channel<LogChunk> → 前端日志面板
//!   前端 Channel GC → send() 失败 → tail task 自动退出（无需显式退订）
//!
//! 启动日志：start_project 在 spawn 前 truncate start.log，故只含本次启动输出。
//! 构建日志：build_project 在跑 mvn 前 truncate build.log，故只含本次构建输出。

pub mod paths;
pub mod tail;

use serde::Serialize;

/// 实时日志推送的数据块。
///
/// - `offset`：本块数据在日志文件中的字节偏移
/// - `text`：新增的文本内容（可能含多行）
#[derive(Debug, Clone, Serialize)]
pub struct LogChunk {
    pub offset: u64,
    pub text: String,
}
