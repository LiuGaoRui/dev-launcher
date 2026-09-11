//! DevLauncher 后端入口
//!
//! 阶段 1：DB 层（migration + models + services + commands）就位。

mod commands;
mod db;
mod error;
mod logs;
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
            // project
            commands::project::list_projects,
            commands::project::get_project,
            commands::project::create_project,
            commands::project::update_project,
            commands::project::delete_project,
            commands::project::reorder_projects,
            // detect
            commands::detect::scan_projects,
            // process
            commands::process::start_project,
            commands::process::stop_project,
            commands::process::build_project,
            commands::process::stop_build,
            commands::process::get_build_status,
            commands::process::probe_statuses,
            // scan_root
            commands::scan_root::list_scan_root_order,
            commands::scan_root::reorder_scan_roots,
            // system
            commands::system::open_url,
            commands::system::get_app_memory,
            // port（端口监控器）
            commands::port::list_listening_ports,
            // port（端口忽略名单）
            commands::port::add_port_ignore,
            commands::port::remove_port_ignore,
            // log
            commands::log::subscribe_log,
            commands::log::unsubscribe_log,
            commands::log::clear_log,
            // cleaner（内存清理器）
            commands::cleaner::scan_dev_processes,
            commands::cleaner::kill_dev_processes,
            commands::cleaner::trim_dev_processes,
            commands::cleaner::get_system_memory,
            // cleaner（进程锁定）
            commands::cleaner::list_cleaner_locks,
            commands::cleaner::add_cleaner_lock,
            commands::cleaner::remove_cleaner_lock,
            commands::cleaner::clear_cleaner_locks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
