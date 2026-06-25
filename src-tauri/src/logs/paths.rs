//! 日志路径计算与日期列举
//!
//! 复用 `spawn::sanitize_name`（已 pub）拼目录名，保证「写日志」（spawn）
//! 与「读/列日志」（本模块）落在同一物理路径，绝不错位。

use std::path::{Path, PathBuf};

use crate::error::AppResult;
use crate::process::spawn::sanitize_name;

/// 项目日志目录：`{logs_root}/{sanitized_name}`（不创建目录，仅计算路径）。
pub fn project_log_dir(logs_root: &Path, project_name: &str) -> PathBuf {
    logs_root.join(sanitize_name(project_name))
}

/// 指定日期的日志文件路径：`{logs_root}/{sanitized_name}/{date}.log`。
///
/// `date` 应为 `YYYYMMDD` 格式（由前端传入，来源 `list_log_dates` 或当天）。
/// 不创建文件，调用方按需处理「文件不存在」。
pub fn date_log_path(logs_root: &Path, project_name: &str, date: &str) -> PathBuf {
    project_log_dir(logs_root, project_name).join(format!("{date}.log"))
}

/// 当天日期字符串 `YYYYMMDD`（本地时区）。
pub fn today() -> String {
    chrono::Local::now().format("%Y%m%d").to_string()
}

/// 列出某项目的全部历史日志日期（扫目录下 `*.log`，按日期降序）。
///
/// 目录不存在视为无日志，返回空 Vec（项目从未启动过日志目录还没建）。
/// 降序：最新日期在前，便于前端日期选择器默认选最近一天。
pub fn list_log_dates(logs_root: &Path, project_name: &str) -> AppResult<Vec<String>> {
    let dir = project_log_dir(logs_root, project_name);

    let read_dir = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(e) => return Err(e.into()),
    };

    let mut dates = Vec::new();
    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        // 仅识别 YYYYMMDD.log 形态：去 .log 后缀，剩 8 位纯数字
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext == "log" && stem.len() == 8 && stem.chars().all(|c| c.is_ascii_digit()) {
            dates.push(stem.to_string());
        }
    }

    // 降序：最新在前
    dates.sort_unstable_by(|a, b| b.cmp(a));
    Ok(dates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_and_path_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project_log_dir(tmp.path(), "HR 系统/后端");
        let path = date_log_path(tmp.path(), "HR 系统/后端", "20260625");
        assert!(dir.to_string_lossy().contains("HR_系统_后端"));
        assert!(path.ends_with("20260625.log"));
    }

    #[test]
    fn list_dates_missing_dir_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dates = list_log_dates(tmp.path(), "不存在项目").unwrap();
        assert!(dates.is_empty());
    }

    #[test]
    fn list_dates_sorted_desc() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = project_log_dir(tmp.path(), "demo");
        std::fs::create_dir_all(&dir).unwrap();
        // 造三个日期日志 + 一个干扰文件
        std::fs::write(dir.join("20260625.log"), b"a").unwrap();
        std::fs::write(dir.join("20260624.log"), b"b").unwrap();
        std::fs::write(dir.join("20260626.log"), b"c").unwrap();
        std::fs::write(dir.join("readme.txt"), b"d").unwrap();
        std::fs::write(dir.join("2026062.log"), b"e").unwrap(); // 7 位，应被过滤

        let dates = list_log_dates(tmp.path(), "demo").unwrap();
        assert_eq!(dates, vec!["20260626", "20260625", "20260624"]);
    }
}
