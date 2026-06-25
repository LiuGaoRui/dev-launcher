//! 全局应用状态

use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::error::AppResult;

/// 全局共享状态
///
/// 阶段 0 只有基础路径，后续阶段补充 ProcessRegistry / DB 连接池等。
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    /// 应用数据目录（如 %APPDATA%\com.devlauncher.app）
    data_dir: std::path::PathBuf,
    /// 日志根目录
    logs_root: std::path::PathBuf,
}

impl AppState {
    pub fn new(app: &AppHandle) -> AppResult<Self> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| crate::error::AppError::Process(format!("获取数据目录失败: {e}")))?;

        let logs_root = data_dir.join("logs");

        // 确保目录存在
        std::fs::create_dir_all(&logs_root)?;

        Ok(Self {
            inner: Arc::new(Inner {
                data_dir,
                logs_root,
            }),
        })
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.inner.data_dir
    }

    pub fn logs_root(&self) -> &std::path::Path {
        &self.inner.logs_root
    }
}
