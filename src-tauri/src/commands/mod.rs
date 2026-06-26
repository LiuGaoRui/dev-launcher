//! 命令薄层模块
//!
//! 每个命令从 `AppHandle` / `AppState` 取所需依赖，转调对应 service。
//! 命令签名对齐 docs/03-命令清单.md。

pub mod detect;
pub mod group;
pub mod log;
pub mod process;
pub mod project;
