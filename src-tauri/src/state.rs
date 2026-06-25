//! 全局应用状态

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

use crate::error::AppResult;
use crate::process::ProcessRegistry;

/// 全局共享状态
///
/// - `data_dir` / `logs_root`：路径常量
/// - `registry`：运行中进程注册表
/// - `system`：持久化 sysinfo System，监控 probe 时 refresh 形成 CPU 基线
///   （sysinfo cpu_usage 需两次 refresh 间隔才准确，故全程复用同一实例）
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    /// 应用数据目录（如 %APPDATA%\com.devlauncher.app）。
    /// 当前仅用于派生 logs_root，保留字段供未来扩展（构建产物目录、DB 路径等）。
    #[allow(dead_code)]
    data_dir: std::path::PathBuf,
    /// 日志根目录（data_dir/logs）
    logs_root: std::path::PathBuf,
    /// 运行中进程注册表
    registry: ProcessRegistry,
    /// 持久化 sysinfo，监控探测用
    system: Mutex<sysinfo::System>,
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
                registry: ProcessRegistry::new(),
                system: Mutex::new(sysinfo::System::new()),
            }),
        })
    }

    #[allow(dead_code)]
    pub fn data_dir(&self) -> &std::path::Path {
        &self.inner.data_dir
    }

    pub fn logs_root(&self) -> &std::path::Path {
        &self.inner.logs_root
    }

    /// 进程注册表（共享引用，内部自带 Mutex）。
    pub fn registry(&self) -> &ProcessRegistry {
        &self.inner.registry
    }

    /// 持久化 sysinfo System（监控探测用）。
    pub fn system(&self) -> &Mutex<sysinfo::System> {
        &self.inner.system
    }
}
