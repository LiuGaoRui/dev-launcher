//! 命令薄层模块
//!
//! 每个命令从 `AppHandle` 取 DB 连接池，转调对应 service。
//! 命令签名对齐 docs/03-命令清单.md。

pub mod group;
pub mod project;
