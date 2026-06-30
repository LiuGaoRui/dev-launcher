//! 进程托管核心
//!
//! 模块结构（见 docs/10-目录结构规范.md）：
//! - `job_object`：Windows Job Object 的 unsafe FFI 封装（杀整树）
//! - `spawn`：tokio::process::Command + stdio→file 重定向
//! - `registry`：ProcessRegistry，运行中进程注册表
//! - `tree`：基于 sysinfo 收集后代 PID（供监控用）
//! - `monitor`：监控探测（CPU/内存/端口，阶段 5）
//! - `build`：构建执行（后台跑完，stdout/stderr 重定向到 build.log）
//! - `kill`：进程树终止公共封装（taskkill /F /T）
//! - `dev_scan`：全系统开发进程扫描 + 智能分类（内存清理器用）

pub mod build;
pub mod dev_scan;
pub mod job_object;
pub mod kill;
pub mod monitor;
pub mod registry;
pub mod spawn;
pub mod tree;

pub use registry::ProcessRegistry;

use serde::Serialize;

/// start_project / restart_project 的返回值（对齐命令清单）。
#[derive(Debug, Clone, Serialize)]
pub struct StartResult {
    pub root_pid: u32,
    pub log_path: String,
    pub started_at: String,
}
