//! port 命令薄层：list_listening_ports + 端口忽略 CRUD。
//!
//! 端口监控器后端：扫描全系统 TCP LISTEN 端口，分类评分后按关注度降序返回。
//! 重点暴露 AI 工具启动后遗留的异常存活服务（suspicious 档）。
//!
//! 忽略名单（port_ignore 表，按端口号持久化）：用户手动认可的可疑端口，
//! 命中后扫描结果统一覆盖为 ignored 分类（不再计入角标），可恢复。
//!
//! 杀进程复用 cleaner 命令组的 kill_dev_processes（taskkill /F /T 杀整树，
//! 含锁定表/自身护栏），本模块不重复提供。

use std::collections::HashSet;

use sqlx::Row;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
use tauri::{AppHandle, Manager, Runtime};

use crate::db;
use crate::error::AppResult;
use crate::process::monitor::collect_tcp_sockets;
use crate::process::port_scan::{scan_listening_ports, ListeningPortInfo, ProjectPortRef};
use crate::services::PortIgnoreService;
use crate::state::AppState;

/// 端口扫描所需的 ProcessRefreshKind。
///
/// 端口监控不展示 CPU（无需 with_cpu 形成基线），但分类与展示依赖
/// cmd/cwd/exe（sysinfo 0.32 默认不采集，必须显式请求）。
fn port_refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::new()
        .with_cmd(UpdateKind::Always)
        .with_cwd(UpdateKind::Always)
        .with_exe(UpdateKind::Always)
        .with_memory()
}

/// 扫描全系统监听端口（TCP LISTEN，v4+v6），分类评分后按 score 降序返回。
///
/// 数据流：DB 拉项目全表（expected_ports 匹配 + 托管判定）→ TCP 全表一次
/// （锁外）→ 持 system 锁刷新进程后调 port_scan::scan_listening_ports。
///
/// 前端：TitleBar 角标低频（~30s）轮询 + 弹窗打开时高频（~5s）轮询。
#[tauri::command]
pub async fn list_listening_ports<R: Runtime>(
    app: AppHandle<R>,
) -> AppResult<Vec<ListeningPortInfo>> {
    let state = app.state::<AppState>();
    let pool = db::pool(&app)?;

    // 1. DB：全部项目的 id/name/expected_ports/last_pid（一次 SQL）
    let rows = sqlx::query("SELECT id, name, expected_ports, last_pid FROM project")
        .fetch_all(&pool)
        .await?;
    let projects: Vec<ProjectPortRef> = rows
        .iter()
        .filter_map(|row| {
            let id: i64 = row.try_get("id").ok()?;
            let name: String = row.try_get("name").ok()?;
            let ports_json: String = row.try_get("expected_ports").ok()?;
            let last_pid: Option<i64> = row.try_get("last_pid").ok()?;
            // 与 probe_statuses 一致：JSON TEXT → Vec<String>，非数字项跳过
            let ports: Vec<u16> = serde_json::from_str::<Vec<String>>(&ports_json)
                .unwrap_or_default()
                .iter()
                .filter_map(|p| p.trim().parse::<u16>().ok())
                .collect();
            Some(ProjectPortRef {
                project_id: id,
                project_name: name,
                ports,
                last_pid: last_pid.and_then(|p| u32::try_from(p).ok()),
            })
        })
        .collect();

    // 2. 忽略名单（port_ignore，按端口号）+ TCP 全表一次查（均不依赖 system，锁外）
    let ignored_ports = PortIgnoreService::list_ignored(&pool).await?;
    let sockets = collect_tcp_sockets();

    // 3. 持锁刷新 + 扫描（registry 托管标记在锁内取，避免与 stop 竞态）
    let self_pid = sysinfo::get_current_pid().ok();
    let result = {
        let registry_ids: HashSet<i64> =
            state.registry().running_ids().into_iter().collect();
        let mut system = state.system().lock().expect("sysinfo mutex poisoned");
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, port_refresh_kind());
        scan_listening_ports(&system, &sockets, &projects, &registry_ids, &ignored_ports, self_pid)
    };

    Ok(result)
}

/// 端口忽略 CRUD（薄层，业务在 PortIgnoreService）。
///
/// 按端口号持久化（端口稳定，进程重启不影响）；同端口号被其他进程占用
/// 仍被压制——语义 = 用户认可该端口号，不再提醒、不计入角标。
/// 未在监听的死条目无害：端口再次被占用时会以 ignored 分类重新出现，可恢复。

/// 忽略一个端口（可疑行「忽略」按钮，重复忽略幂等）。
#[tauri::command]
pub async fn add_port_ignore<R: Runtime>(app: AppHandle<R>, port: u16) -> AppResult<()> {
    let pool = db::pool(&app)?;
    PortIgnoreService::add(&pool, port).await
}

/// 恢复一个端口（「已忽略」折叠区「恢复」按钮）。
#[tauri::command]
pub async fn remove_port_ignore<R: Runtime>(app: AppHandle<R>, port: u16) -> AppResult<()> {
    let pool = db::pool(&app)?;
    PortIgnoreService::remove(&pool, port).await
}
