//! DevLauncher 后端入口
//!
//! 阶段 1：DB 层（migration + models + services + commands）就位。

mod commands;
mod db;
mod error;
mod models;
mod process;
mod services;
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
                .add_migrations(db::DB_CONN_URL, db::migrations::migrations())
                .build(),
        )
        .setup(|app| {
            // 初始化全局状态
            let state = AppState::new(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // group
            commands::group::list_groups,
            commands::group::create_group,
            commands::group::update_group,
            commands::group::delete_group,
            // project
            commands::project::list_projects,
            commands::project::get_project,
            commands::project::create_project,
            commands::project::update_project,
            commands::project::delete_project,
            // process
            commands::process::start_project,
            commands::process::stop_project,
            commands::process::restart_project,
            commands::process::probe_statuses,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
