//! 日志模块：实时 tail（启动日志 + 构建日志）
//!
//! 模块结构：
//! - `paths`：日志路径计算（`{logs_root}/{project_id}/{start|build}.log`）
//! - `subscription`：订阅注册表，显式取消 tail task（ADR-006）
//! - `tail`：tokio tail task，轮询日志文件增量，通过 ipc::Channel 推送
//!
//! 数据流：
//!   子进程 stdout/stderr → append 落盘 {logs_root}/{project_id}/{start|build}.log
//!   → tail task 轮询读增量 → Channel<LogChunk>（带 sub_id）→ 前端日志面板
//!   前端切换订阅/离开页面 → unsubscribe_log(sub_id) → task 检查取消标志后退出
//!
//! 隔离保证：目录按项目 id（不同项目绝不共用文件）；每次订阅有唯一 sub_id，
//! 前端据此丢弃过期通道的推送（详见 ADR-006）。
//!
//! 启动日志：start_project 在 spawn 前 truncate start.log，故只含本次启动输出。
//! 构建日志：build_project 在跑构建命令前 truncate build.log，故只含本次构建输出。

pub mod paths;
pub mod subscription;
pub mod tail;

use serde::Serialize;

/// 实时日志推送的数据块。
///
/// - `sub_id`：产生本块的订阅 id，前端据此丢弃过期订阅的推送
/// - `offset`：本块数据在日志文件中的字节偏移
/// - `text`：新增的文本内容（可能含多行）
#[derive(Debug, Clone, Serialize)]
pub struct LogChunk {
    pub sub_id: u64,
    pub offset: u64,
    pub text: String,
}
