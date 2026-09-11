//! 日志路径计算
//!
//! 目录以**项目 id**（DB 主键）为键，而非项目名：项目名无 UNIQUE 约束，同名或
//! sanitize 后同名的项目会共用同一目录，造成日志互相覆盖、交错追加。
//! id 唯一且与项目名解耦，项目改名也不会丢日志。
//!
//! 日志按类型分两个固定文件，无日期分片：
//! - 启动日志：`{logs_root}/{project_id}/start.log`
//! - 构建日志：`{logs_root}/{project_id}/build.log`
//! 每次启动/构建前由调用方 truncate，故只保留「本次」输出（实时订阅即可完整看到）。

use std::path::{Path, PathBuf};

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

/// 项目日志目录：`{logs_root}/{project_id}`（不创建目录，仅计算路径）。
pub fn project_log_dir(logs_root: &Path, project_id: i64) -> PathBuf {
    logs_root.join(project_id.to_string())
}

/// 指定类型的日志文件路径：`{logs_root}/{project_id}/{start|build}.log`。
///
/// 不创建文件，调用方按需处理「文件不存在」。spawn/build 写日志用的也是本函数，
/// 保证读写路径一致。
pub fn log_path_of(logs_root: &Path, project_id: i64, log_type: LogType) -> PathBuf {
    project_log_dir(logs_root, project_id).join(log_type.file_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_layout_by_type() {
        let tmp = std::path::PathBuf::from("/tmp/logs");
        let start = log_path_of(&tmp, 42, LogType::Start);
        let build = log_path_of(&tmp, 42, LogType::Build);
        assert_eq!(start.parent().unwrap(), project_log_dir(&tmp, 42));
        assert_eq!(start.parent().unwrap().file_name().unwrap(), "42");
        assert!(start.ends_with("start.log"));
        assert!(build.ends_with("build.log"));
    }

    #[test]
    fn distinct_projects_never_share_a_directory() {
        let tmp = std::path::PathBuf::from("/tmp/logs");
        assert_ne!(
            project_log_dir(&tmp, 1),
            project_log_dir(&tmp, 2),
            "不同项目 id 必须落不同目录（这是修复同名项目日志串写的关键）"
        );
    }

    #[test]
    fn parse_recognizes_known_types() {
        assert_eq!(LogType::parse("start"), LogType::Start);
        assert_eq!(LogType::parse("build"), LogType::Build);
        // 非法值兜底为 Start
        assert_eq!(LogType::parse("xxx"), LogType::Start);
    }
}
