//! 全局错误类型

use serde::Serialize;
use thiserror::Error;

/// 所有 command 的统一错误
///
/// `AlreadyRunning` / `NotRunning` 预留给阶段 2 进程托管使用。
#[derive(Debug, Error)]
#[allow(dead_code)] // 部分变体为后续阶段预留
pub enum AppError {
    #[error("项目不存在: id={0}")]
    ProjectNotFound(i64),

    #[error("项目正在运行: id={0}")]
    AlreadyRunning(i64),

    #[error("项目未运行: id={0}")]
    NotRunning(i64),

    #[error("分组不存在: id={0}")]
    GroupNotFound(i64),

    #[error("分组名称已存在: {0}")]
    GroupNameExists(String),

    #[error("项目名称已存在: {0}")]
    ProjectNameExists(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("数据库错误: {0}")]
    Database(String),

    #[error("SQL 错误: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("进程错误: {0}")]
    Process(String),

    #[error("JSON 序列化错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Tauri 错误: {0}")]
    Tauri(#[from] tauri::Error),
}

// Tauri 自动把错误序列化为前端可读的格式
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
