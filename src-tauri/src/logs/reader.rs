//! 历史日志分页读取
//!
//! 简单的字节偏移分页：从 `offset` 处 seek，读最多 `limit` 字节。
//! 不按行对齐（按字节切片），前端负责展示。`next_offset` 指向下一页起点，
//! 为 None 表示已读到文件末尾。

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::AppResult;
use crate::logs::LogHistoryPage;

/// 从 `log_path` 的 `offset` 字节处读取最多 `limit` 字节。
///
/// - 文件不存在 / offset 超过文件大小：返回空 page（`total_size` 仍如实反映文件大小）
/// - 正常：`data` 为读到的内容（可能少于 limit，表示到末尾），
///   `next_offset` = `offset + data.len()`，若等于 `total_size` 则为 None
pub fn read_history(log_path: &Path, offset: u64, limit: u64) -> AppResult<LogHistoryPage> {
    let file = match std::fs::File::open(log_path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LogHistoryPage {
                total_size: 0,
                data: String::new(),
                next_offset: None,
            });
        }
        Err(e) => return Err(e.into()),
    };

    let total_size = file.metadata()?.len();

    // offset 已超过文件末尾 → 空
    if offset >= total_size {
        return Ok(LogHistoryPage {
            total_size,
            data: String::new(),
            next_offset: None,
        });
    }

    let mut file = file;
    file.seek(SeekFrom::Start(offset))?;

    let cap = std::cmp::min(limit, total_size - offset) as usize;
    let mut buf = vec![0u8; cap];
    let n = read_full(&mut file, &mut buf)?;
    buf.truncate(n);

    let data = String::from_utf8_lossy(&buf).into_owned();
    let next = offset + n as u64;
    let next_offset = if next >= total_size { None } else { Some(next) };

    Ok(LogHistoryPage {
        total_size,
        data,
        next_offset,
    })
}

/// 读满 buf，返回实际读取字节数（遇到 EOF 提前结束）。
fn read_full(file: &mut std::fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break, // EOF
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_tmp(content: &str) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("app.log"), content).unwrap();
        tmp
    }

    #[test]
    fn missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let page = read_history(&tmp.path().join("nope.log"), 0, 100).unwrap();
        assert_eq!(page.total_size, 0);
        assert!(page.data.is_empty());
        assert_eq!(page.next_offset, None);
    }

    #[test]
    fn full_read_then_eof() {
        let tmp = write_tmp("hello world");
        let path = tmp.path().join("app.log");

        let page = read_history(&path, 0, 1000).unwrap();
        assert_eq!(page.total_size, 11);
        assert_eq!(page.data, "hello world");
        assert_eq!(page.next_offset, None); // 读完即末尾
    }

    #[test]
    fn paged_read_has_next() {
        let tmp = write_tmp("0123456789"); // 10 字节
        let path = tmp.path().join("app.log");

        let page = read_history(&path, 0, 4).unwrap();
        assert_eq!(page.data, "0123");
        assert_eq!(page.next_offset, Some(4));

        let page2 = read_history(&path, 4, 4).unwrap();
        assert_eq!(page2.data, "4567");
        assert_eq!(page2.next_offset, Some(8));

        let page3 = read_history(&path, 8, 4).unwrap();
        assert_eq!(page3.data, "89");
        assert_eq!(page3.next_offset, None); // 到末尾
    }

    #[test]
    fn offset_past_end_returns_empty() {
        let tmp = write_tmp("ab");
        let path = tmp.path().join("app.log");
        let page = read_history(&path, 100, 10).unwrap();
        assert_eq!(page.total_size, 2);
        assert!(page.data.is_empty());
        assert_eq!(page.next_offset, None);
    }
}
