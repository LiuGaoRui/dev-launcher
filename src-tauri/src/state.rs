//! 全局应用状态

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

use crate::error::AppResult;
use crate::process::ProcessRegistry;

/// 后台构建状态（供前端轮询展示构建按钮状态图标）。
///
/// 每个项目独立一份（支持多项目同时构建）。running=true 表示进行中；
/// running=false 时 exit_code 表示上次结果（0 成功，非 0 失败）。
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct BuildState {
    /// 是否构建中
    pub running: bool,
    /// 退出码（构建结束才填，默认 0；running 期间无意义）
    pub exit_code: i32,
    /// 耗时（ms，构建结束才填）
    pub duration_ms: u64,
    /// 错误信息（命令本身抛错时填，如未配 build_cmd、启动失败）
    #[serde(default)]
    pub error: String,
}

/// 全局共享状态
///
/// - `data_dir` / `logs_root`：路径常量
/// - `registry`：运行中进程注册表
/// - `builds`：后台构建状态表（项目 id → BuildState）
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
    /// 后台构建状态表：project_id → BuildState（多项目可同时构建）
    builds: Mutex<HashMap<i64, BuildState>>,
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
                builds: Mutex::new(HashMap::new()),
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

    /// 后台构建状态表（共享引用，内部 Mutex 保护）。
    pub fn builds(&self) -> &Mutex<HashMap<i64, BuildState>> {
        &self.inner.builds
    }

    /// 持久化 sysinfo System（监控探测用）。
    pub fn system(&self) -> &Mutex<sysinfo::System> {
        &self.inner.system
    }
}
