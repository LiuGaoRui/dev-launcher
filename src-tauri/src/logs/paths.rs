//! 日志路径计算
//!
//! 复用 `spawn::sanitize_name`（已 pub）拼目录名，保证「写日志」（spawn/build）
//! 与「读日志」（本模块 tail）落在同一物理路径，绝不错位。
//!
//! 日志按类型分两个固定文件，无日期分片：
//! - 启动日志：`{logs_root}/{name}/start.log`
//! - 构建日志：`{logs_root}/{name}/build.log`
//! 每次启动/构建前由调用方 truncate，故只保留「本次」输出（实时订阅即可完整看到）。

use std::path::{Path, PathBuf};

use crate::process::spawn::sanitize_name;

/// 日志类型（对齐前端 log_type 参数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogType {
    Start,
    Build,
}

impl LogType {
    /// 从命令参数解析；非法值视为 Start（兜底，保证订阅不直接报错）。
    pub fn parse(s: &str) -> Self {
        match s {
            "build" => LogType::Build,
            _ => LogType::Start,
        }
    }

    /// 对应的固定文件名（不含目录）。
    fn file_name(self) -> &'static str {
        match self {
            LogType::Start => "start.log",
            LogType::Build => "build.log",
        }
    }
}

/// 项目日志目录：`{logs_root}/{sanitized_name}`（不创建目录，仅计算路径）。
pub fn project_log_dir(logs_root: &Path, project_name: &str) -> PathBuf {
    logs_root.join(sanitize_name(project_name))
}

/// 指定类型的日志文件路径：`{logs_root}/{sanitized_name}/{start|build}.log`。
///
/// 不创建文件，调用方按需处理「文件不存在」。spawn/build 写日志用的也是本函数，
/// 保证读写路径一致。
pub fn log_path_of(logs_root: &Path, project_name: &str, log_type: LogType) -> PathBuf {
    project_log_dir(logs_root, project_name).join(log_type.file_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_layout_by_type() {
        let tmp = std::path::PathBuf::from("/tmp/logs");
        let start = log_path_of(&tmp, "HR 系统/后端", LogType::Start);
        let build = log_path_of(&tmp, "HR 系统/后端", LogType::Build);
        assert!(start.to_string_lossy().contains("HR_系统_后端"));
        assert!(start.ends_with("start.log"));
        assert!(build.ends_with("build.log"));
    }

    #[test]
    fn parse_recognizes_known_types() {
        assert_eq!(LogType::parse("start"), LogType::Start);
        assert_eq!(LogType::parse("build"), LogType::Build);
        // 非法值兜底为 Start
        assert_eq!(LogType::parse("xxx"), LogType::Start);
    }
}
