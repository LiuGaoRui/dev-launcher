//! 进程监控：探测运行中项目的 CPU / 内存 / 端口状态。
//!
//! 设计（见开发计划 §5.4/§5.6、ADR-003）：
//! - **指标聚合整树**：用 `collect_tree` 聚合根+所有后代 PID 的 CPU/内存，
//!   避免 npm/mvn 包装进程（启动后会 fork 真正的工作进程）显示 0。
//! - **端口归属校验**（风险 R3）：端口 Listen 时再校验占用者 PID 是否属于本项目进程树，
//!   杜绝「8080 被别的程序占了，本项目却误报端口正常」。
//! - **三态状态机**：stopped / running / running_abnormal
//!   （进程在但预期端口未监听或被非本项目占用）。
//!
//! sysinfo 0.32 关键约定：
//! - `cpu_usage()` 需两次 refresh（间隔 > MINIMUM_CPU_UPDATE_INTERVAL）才准确，
//!   故 AppState 持久化 System，每 3s probe 时 refresh 一次形成基线。
//! - `memory()` 返回 bytes（RSS）。
//!
//! netstat2 0.9 关键约定：
//! - `get_sockets_info` 较重（遍历系统全表），一次调用后对所有项目复用。

use std::collections::HashSet;

use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
};
use serde::Serialize;
use sysinfo::{Pid, System};
use tracing::warn;

use crate::process::registry::ProcessSnapshot;
use crate::process::tree::collect_tree;

// ===== 序列化结构（对齐前端 types/monitor.ts，serde snake_case） =====

/// 项目健康状态（对齐开发计划 §5.2 状态机，简化为三态）。
///
/// - `Stopped`：未运行（registry 中无此项目）
/// - `Running`：进程存活，且（无预期端口 或 全部端口 listening && owned）
/// - `RunningAbnormal`：进程存活，但存在预期端口未监听 / 被非本项目占用
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Stopped,
    Running,
    RunningAbnormal,
}

/// 单个预期端口的探测结果。
#[derive(Debug, Clone, Serialize)]
pub struct PortStatus {
    /// 端口号字符串（与 project.expected_ports 一致，保留原始格式）
    pub port: String,
    /// 是否处于 TCP LISTEN
    pub listening: bool,
    /// 占用者 PID 是否属于本项目进程树（仅在 listening 时有意义）
    pub owned: bool,
}

/// 单个项目的完整探测结果（probe_statuses 命令返回元素）。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatus {
    pub project_id: i64,
    pub health: HealthStatus,
    /// 根进程 PID（运行中时来自 registry snapshot）
    pub pid: Option<u32>,
    /// 整树 CPU 占用百分比（多核机器可能 > 100）
    pub cpu_percent: f32,
    /// 整树内存（RSS）字节数
    pub memory_bytes: u64,
    /// 各预期端口的探测结果
    pub ports: Vec<PortStatus>,
    /// 启动时间（来自 registry snapshot）
    pub started_at: Option<String>,
}

/// 对单个运行中项目探测（registry snapshot 必存在）。
///
/// `expected_ports` 为项目配置的端口列表（字符串形式，如 ["8080", "5173"]）。
/// `sockets` 为一次 `get_sockets_info` 的全表缓存，避免每个端口重复查。
pub fn probe_one(
    project_id: i64,
    snapshot: &ProcessSnapshot,
    expected_ports: &[String],
    system: &System,
    sockets: &[netstat2::SocketInfo],
) -> ProjectStatus {
    // 收集整树 PID（含 root）用于 CPU/内存聚合 + 端口归属校验
    let tree_vec: Vec<u32> = collect_tree(snapshot.pid, system);

    // CPU / 内存整树求和（迭代 Vec 即可，无需 HashSet）
    let (cpu_percent, memory_bytes) = aggregate_tree(&tree_vec, system);

    let tree_pids: HashSet<u32> = tree_vec.into_iter().collect();

    // 端口探测：对每个预期端口，查 sockets 是否有匹配的 LISTEN，并校验归属
    let ports = expected_ports
        .iter()
        .map(|port| {
            let (listening, owned) = check_port(port, &tree_pids, sockets);
            PortStatus {
                port: port.clone(),
                listening,
                owned,
            }
        })
        .collect::<Vec<_>>();

    // 健康判定：无预期端口 → 运行中；否则要求全部 listening && owned
    let health = if ports.is_empty() {
        HealthStatus::Running
    } else if ports.iter().all(|p| p.listening && p.owned) {
        HealthStatus::Running
    } else {
        HealthStatus::RunningAbnormal
    };

    ProjectStatus {
        project_id,
        health,
        pid: Some(snapshot.pid),
        cpu_percent,
        memory_bytes,
        ports,
        started_at: Some(snapshot.started_at.clone()),
    }
}

/// 对全树 PID 聚合 CPU（f32 求和）/ 内存（bytes 求和）。
///
/// CPU 说明：sysinfo `cpu_usage()` 为「自上次 refresh 以来的平均」，
/// 单进程值可能瞬时偏高，整树求和后作为粗略指标足够（前端只展示，不做精确告警）。
fn aggregate_tree(tree_pids: &[u32], system: &System) -> (f32, u64) {
    let mut cpu = 0.0f32;
    let mut mem = 0u64;
    for &pid_u32 in tree_pids {
        if let Some(proc) = system.process(Pid::from_u32(pid_u32)) {
            cpu += proc.cpu_usage();
            mem += proc.memory();
        }
    }
    (cpu, mem)
}

/// 查某端口是否监听，以及占用者是否属于本项目进程树。
///
/// 返回 `(listening, owned)`：
/// - `listening`：存在 TCP LISTEN 且 local_port == port
/// - `owned`：listening 时，至少一个 associated_pid 在 tree_pids 内
///
/// 端口字符串解析失败时视为未监听（容错：用户可能填了非数字端口）。
fn check_port(
    port: &str,
    tree_pids: &HashSet<u32>,
    sockets: &[netstat2::SocketInfo],
) -> (bool, bool) {
    let port_u16: u16 = match port.trim().parse() {
        Ok(n) => n,
        Err(_) => return (false, false),
    };

    let mut listening = false;
    let mut owned = false;
    for si in sockets {
        if let ProtocolSocketInfo::Tcp(tcp) = &si.protocol_socket_info {
            if tcp.local_port == port_u16 && tcp.state == TcpState::Listen {
                listening = true;
                // 校验归属：占用者 PID 任一在进程树内即视为本项目占用
                if si.associated_pids.iter().any(|p| tree_pids.contains(p)) {
                    owned = true;
                    break; // 已确认归属，无需继续
                }
            }
        }
    }
    (listening, owned)
}

/// 端口发现：查某端口是否处于 LISTEN，返回占用者 PID（首个 associated_pid）。
///
/// 与 `check_port` 的区别：`check_port` 已知本项目进程树、仅校验归属；
/// 本函数用于**未知**占用者的发现场景——外部启动的进程（如 VSCode 终端 npm dev）
/// 没有写入 registry / DB last_pid，需通过端口反查其 PID 才能接管。
///
/// 返回 None 的情况：端口非数字、无 LISTEN、或 netstat 未给出关联 PID。
pub fn find_port_owner(port: &str, sockets: &[netstat2::SocketInfo]) -> Option<u32> {
    let port_u16: u16 = port.trim().parse().ok()?;
    for si in sockets {
        if let ProtocolSocketInfo::Tcp(tcp) = &si.protocol_socket_info {
            if tcp.local_port == port_u16 && tcp.state == TcpState::Listen {
                // 取首个关联 PID 即可（同一端口多 PID 极罕见，且仅需定位进程）
                if let Some(pid) = si.associated_pids.first() {
                    return Some(*pid);
                }
            }
        }
    }
    None
}

/// 查询系统全部 TCP（v4+v6）socket 一次，供所有项目复用。
///
/// 失败时返回空表（监控不应因单次 netstat 失败中断整个 probe）。
pub fn collect_tcp_sockets() -> Vec<netstat2::SocketInfo> {
    match get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP,
    ) {
        Ok(v) => v,
        Err(e) => {
            warn!("netstat2 get_sockets_info 失败，端口探测降级为未知: {e}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn snap(pid: u32) -> ProcessSnapshot {
        ProcessSnapshot {
            pid,
            log_path: PathBuf::from("/tmp/x.log"),
            started_at: "2026-01-01 00:00:00".into(),
        }
    }

    #[test]
    fn aggregate_tree_empty_for_unknown_pid() {
        let system = System::new();
        let pids = vec![0xFFFF_FFFFu32];
        let (cpu, mem) = aggregate_tree(&pids, &system);
        assert_eq!((cpu, mem), (0.0, 0));
    }

    #[test]
    fn check_port_returns_false_for_non_numeric() {
        let pids = HashSet::<u32>::new();
        let (listening, owned) = check_port("abc", &pids, &[]);
        assert!(!listening);
        assert!(!owned);
    }

    #[test]
    fn find_port_owner_none_for_non_numeric_or_empty() {
        assert_eq!(find_port_owner("abc", &[]), None);
        assert_eq!(find_port_owner("8080", &[]), None);
    }

    #[test]
    fn probe_one_running_when_no_expected_ports() {
        let system = System::new();
        let sockets = collect_tcp_sockets();
        let snap = snap(0xFFFF_FFFF);
        let status = probe_one(1, &snap, &[], &system, &sockets);
        assert_eq!(status.health, HealthStatus::Running);
        assert_eq!(status.pid, Some(0xFFFF_FFFF));
        assert!(status.ports.is_empty());
        // 整树 PID 不存在 → 资源 0
        assert_eq!(status.cpu_percent, 0.0);
        assert_eq!(status.memory_bytes, 0);
    }
}
