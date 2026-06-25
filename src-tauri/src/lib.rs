//! DevLauncher 后端入口
//!
//! MVP 阶段 0：最小可运行骨架。

mod error;
mod state;

use tauri::Manager;
use state::AppState;

// ===== Tauri 应用启动 =====

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dev_launcher=debug,info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:devlauncher.db", migrations())
                .build(),
        )
        .setup(|app| {
            // 初始化全局状态
            let state = AppState::new(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 注册数据库迁移（阶段 1 填充实际 SQL）
fn migrations() -> Vec<tauri_plugin_sql::Migration> {
    vec![]
}
