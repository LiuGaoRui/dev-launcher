//! 本机监听端口扫描 + 分类评分（端口监控器）。
//!
//! 与 `dev_scan.rs` 的区别：dev_scan 从**进程**侧扫描（按 java/node 运行时识别）；
//! 本模块从 TCP LISTEN 全表出发扫描**端口**，为每个监听端口判定分类（PortCategory）
//! 并计算关注度分数（score，0-100），按分数降序返回。
//!
//! 设计目标：突出「AI 编程工具启动后遗留的异常存活服务」，压低系统端口 /
//! 环境服务 / 本工具已配置项目端口的存在感。
//!
//! 分类判定按固定优先级（前者优先，保证可解释、无歧义）：
//! 1. owner 不可得（内核监听）/ 系统进程 / 本应用自身 → System
//! 2. 环境服务进程名（mysqld/postgres/redis…）→ EnvService
//! 3. 端口命中项目 expected_ports → KnownProject（托管运行中）/ KnownProjectExternal（外部启动）
//! 4. 知名环境服务端口（3306/6379/9092…，兜底覆盖 java 跑的 Kafka/ES）→ EnvService
//! 5. dev 运行时（node/java/python…）→ Suspicious（孤立/随机端口）或 Dev（知名端口 + IDE 父链）
//! 6. 其余 → Other
//!
//! 展示排序按 score 降序：Suspicious 70+、Dev 50、Other 30、
//! KnownProjectExternal 20、KnownProject 15、EnvService 10、System 0。
//! 另有 Ignored 档（score 0）：命中 port_ignore 忽略名单的端口统一覆盖
//! 为该分类（用户显式意志优先于所有推断），不再计入可疑角标。
//!
//! 复用 dev_scan 的进程识别（is_ide_process/is_orphan）与展示信息提取
//! （display_title / cmdline_summary / project_path）。

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use netstat2::{ProtocolSocketInfo, SocketInfo, TcpState};
use serde::Serialize;
use sysinfo::{Pid, System};

use crate::process::dev_scan::{
    classify, extract_cmdline_summary, extract_display_title, extract_main_script,
    extract_project_path, is_dev_process, is_ide_process, is_orphan, unix_to_iso_string,
};
use crate::process::tree::{build_parent_map, collect_descendants_with_map};

// ===== 序列化结构（对齐前端 types/port.ts，serde snake_case） =====

/// 端口关注度分类（数值区间见模块顶部表格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortCategory {
    /// 可疑：开发运行时监听非项目配置端口（AI 遗留 dev server / 测试服务的典型特征）
    Suspicious,
    /// 开发端口：知名 dev 端口 + 有 IDE 父链（正常开发中）
    Dev,
    /// 其他：无法归类的非系统进程
    Other,
    /// 已配置-外部启动：命中项目 expected_ports，但进程非本工具托管
    KnownProjectExternal,
    /// 已配置-托管：命中项目 expected_ports，且该项目本工具托管运行中
    KnownProject,
    /// 环境服务：mysqld/postgres/redis/docker 等常驻服务，或知名服务端口（3306/6379/9092…）
    EnvService,
    /// 系统端口：svchost/lsass 等 Windows 系统进程监听，或 owner 不可得（内核 http.sys 等）
    System,
    /// 已忽略：用户手动认可该端口号（port_ignore 名单命中），覆盖任何推断分类，
    /// 不再提醒、不计入角标；仍会出现在扫描结果中（「已忽略」折叠区，可恢复）
    Ignored,
}

/// 项目端口引用（命令层从 DB 拉取后传入，port_scan 不直接访问 DB）。
/// 是否托管运行中由 `scan_listening_ports` 的 `registry_ids` 参数判定。
pub struct ProjectPortRef {
    pub project_id: i64,
    pub project_name: String,
    /// 解析成功的预期端口号（非数字配置项被跳过）
    pub ports: Vec<u16>,
    /// DB last_pid（用于判定项目是否外部接管运行中）
    pub last_pid: Option<u32>,
}

/// 单个监听端口的扫描结果（list_listening_ports 命令返回元素）。
#[derive(Debug, Clone, Serialize)]
pub struct ListeningPortInfo {
    /// 监听端口号
    pub port: u16,
    /// 监听地址列表（去重合并 v4/v6，如 "0.0.0.0:8080" / "[::]:8080"）
    pub addresses: Vec<String>,
    /// 是否监听所有网卡（0.0.0.0 / [::]，对外暴露）
    pub listen_all_interfaces: bool,
    /// 占用者 PID（内核监听等场景不可得，为 None）
    pub owner_pid: Option<u32>,
    /// 进程名（如 node.exe）
    pub process_name: String,
    /// 可执行文件路径
    pub exe: String,
    /// 用户友好的展示名（复用 dev_scan 提取，如「vite dev server」「server.js (node)」）
    pub display_title: String,
    /// 命令行语义化摘要（复用 dev_scan 提取）
    pub cmdline_summary: String,
    /// 关联的项目/工作目录路径（复用 dev_scan 提取），可能为空
    pub project_path: String,
    /// 进程启动时间（ISO 字符串，不可得为 None）
    pub started_at: Option<String>,
    /// 占用者 RSS（字节）
    pub memory_bytes: u64,
    /// 关注度分类
    pub category: PortCategory,
    /// 关注度分数（0-100，降序即列表排序）
    pub score: u32,
    /// 父祖链无 IDE（AI CLI / 终端启动进程的典型特征，仅 dev 运行时有意义）
    pub orphan: bool,
    /// 是否为随机高位端口（非知名 dev 端口，测试框架常用）
    pub random_port: bool,
    /// 命中的本工具项目名（端口命中 expected_ports 时）
    pub matched_project_name: Option<String>,
    /// 命中项目是否本工具托管运行中（false = 外部/AI 启动）
    pub matched_project_managed: bool,
    /// 整树 PID 列表（前端「结束进程」时原样回传 kill_dev_processes）
    pub tree_pid_list: Vec<u32>,
}

// ===== 静态识别表 =====

/// Windows 系统服务进程名（监听端口一律归 System 折叠置底）。
const SYSTEM_PROCESS_NAMES: &[&str] = &[
    "svchost.exe", "lsass.exe", "services.exe", "system", "system.exe", "csrss.exe",
    "wininit.exe", "winlogon.exe", "smss.exe", "dwm.exe", "runtimebroker.exe",
    "searchindexer.exe", "searchapp.exe", "searchhost.exe", "startmenuexperiencehost.exe",
    "shellexperiencehost.exe", "applicationframehost.exe", "backgroundtaskhost.exe",
    "wmiprvse.exe", "spoolsv.exe", "sihost.exe", "taskhostw.exe", "ctfmon.exe",
    "fontdrvhost.exe", "msdtc.exe", "wsmprovhost.exe", "wudfhost.exe", "audiodg.exe",
    "dllhost.exe", "mdnsresponder.exe", "appinfo.exe", "inetinfo.exe", "registry.exe",
];

/// 环境服务进程名（常驻中间件/数据库，归 EnvService 静默展示）。
const ENV_SERVICE_NAMES: &[&str] = &[
    "mysqld.exe", "mysqld", "mariadbd.exe", "postgres.exe", "postgres",
    "redis-server.exe", "redis-server", "redis.exe", "mongod.exe", "mongos.exe",
    "nginx.exe", "nginx", "httpd.exe", "apache2.exe", "sqlservr.exe", "sqlbrowser.exe",
    "etcd.exe", "etcd", "consul.exe", "vault.exe", "minio.exe", "minio",
    "influxd.exe", "clickhouse-server.exe", "cockroach.exe",
    "beam.smp", // RabbitMQ（Erlang 运行时）
    "w3wp.exe", // IIS 工作进程
    // Docker Desktop 全家桶
    "com.docker.backend.exe", "com.docker.service", "com.docker.dev-envs.exe",
    "vpnkit.exe", "wslrelay.exe", "wslhost.exe", "docker-proxy.exe",
];

/// 知名环境服务端口（按端口号兜底归类，覆盖 java 跑的 Kafka/ES 等场景）。
/// 注意不得与 KNOWN_DEV_PORTS 重叠（歧义端口如 9000 两表都不收）。
const ENV_SERVICE_PORTS: &[u16] = &[
    1433, 1434, // SQL Server
    3306, 33060, // MySQL + X Protocol
    5432, // PostgreSQL
    6379, // Redis
    27017, 27018, 27019, // MongoDB
    11211, // Memcached
    5672, 15672, // RabbitMQ AMQP + 管理台
    2181, 2888, 3888, // ZooKeeper
    9092, 9094, // Kafka
    9200, 9300, // Elasticsearch
    8500, 8300, // Consul
    8848, // Nacos
];

/// 知名开发端口（vite/next/spring 等默认端口；dev 运行时占用视为正常开发）。
const KNOWN_DEV_PORTS: &[u16] = &[
    3000, 3001, // node / react / next
    4000, 4200, 4173, // phoenix / ember / vite preview
    5000, 5001, // flask / dotnet
    5173, 5174, 5175, // vite
    6006, // storybook
    8000, 8001, // django / uvicorn
    8080, 8081, 8082, 8090, 8091, // spring / tomcat
    8888, 8889, // jupyter
    9229, 9230, // node inspector
];

/// dev 运行时进程名（dev_scan 的 is_dev_process 覆盖 java/node 系，此处扩展）。
const DEV_RUNTIME_EXTRA_NAMES: &[&str] = &[
    "python.exe", "python", "python3.exe", "python3", "pythonw.exe", "py.exe",
    "go.exe", "go", "dotnet.exe", "dotnet", "deno.exe", "deno", "bun.exe", "bun",
    "php.exe", "php", "php-cgi.exe", "ruby.exe", "ruby",
];

/// dev 运行时 exe 路径后缀（覆盖包装器场景）。
const DEV_RUNTIME_EXE_SUFFIXES: &[&str] = &[
    "\\python.exe", "\\pythonw.exe", "\\go.exe", "\\dotnet.exe", "\\deno.exe",
    "\\bun.exe", "\\php.exe",
];

// ===== suspicious 加分项（全部可解释） =====

const SUSPICIOUS_BASE: u32 = 70;
/// 孤立进程（父祖链无 IDE——AI CLI / 终端直接启动的典型特征）
const BONUS_ORPHAN: u32 = 8;
/// 随机高位端口（非知名 dev 端口，测试框架常随机分配）
const BONUS_RANDOM_PORT: u32 = 8;
/// 监听 0.0.0.0/[::]（对外暴露）
const BONUS_EXPOSED: u32 = 4;
/// 存活超过 1 小时（遗留越久越可疑）
const BONUS_LONG_LIVED_SECS: u64 = 3600;
const BONUS_LONG_LIVED: u32 = 5;

// ===== 主扫描 =====

/// 按端口聚合的中间结构。
#[derive(Default)]
struct PortAgg {
    addresses: Vec<String>,
    listen_all: bool,
    /// 首个关联 PID（同端口多占用者极罕见）
    owner_pid: Option<u32>,
}

/// 扫描全系统 TCP LISTEN 端口，分类评分后按 score 降序返回。
///
/// - `system` 应已 refresh（含 cmd/cwd/exe，由命令层持锁刷新）
/// - `sockets` 为一次 `collect_tcp_sockets()` 全表
/// - `projects` 为 DB 全部项目的端口引用（expected_ports 匹配 + 托管判定）
/// - `registry_ids` 为 registry 中运行中的项目 id 集合
/// - `ignored_ports` 为 port_ignore 名单（用户手动认可的端口号），
///   命中的端口无论推断为何分类一律覆盖为 Ignored / score 0
/// - `self_pid` 为本应用自身 PID（归 System 隐藏，防误杀）
pub fn scan_listening_ports(
    system: &System,
    sockets: &[SocketInfo],
    projects: &[ProjectPortRef],
    registry_ids: &HashSet<i64>,
    ignored_ports: &HashSet<u16>,
    self_pid: Option<Pid>,
) -> Vec<ListeningPortInfo> {
    // 1. LISTEN 过滤 + 按端口聚合（v4/v6 地址合并）
    let agg = aggregate_listeners(sockets);

    // 2. expected_ports 索引：端口号 → 项目引用（同端口多项目取首个）
    let mut expected_index: HashMap<u16, &ProjectPortRef> = HashMap::new();
    for p in projects {
        for port in &p.ports {
            expected_index.entry(*port).or_insert(p);
        }
    }

    // 3. IDE 进程集合（孤立判定用）+ parent 索引（整树收集用），各建一次
    let ide_pids: HashSet<u32> = system
        .processes()
        .iter()
        .filter(|(_, proc)| {
            let n = proc.name().to_string_lossy().to_ascii_lowercase();
            let e = proc
                .exe()
                .map(|x| x.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            is_ide_process(&n, &e, proc)
        })
        .map(|(pid, _)| pid.as_u32())
        .collect();
    let parent_map = build_parent_map(system);

    // 4. 逐端口分类评分
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut results: Vec<ListeningPortInfo> = Vec::with_capacity(agg.len());
    for (port, a) in agg {
        let mut info = build_port_info(
            port,
            a,
            system,
            &expected_index,
            registry_ids,
            &ide_pids,
            &parent_map,
            self_pid,
            now_secs,
        );
        // 用户显式忽略优先于所有推断：命中名单 → Ignored / score 0，
        // 沉到排序末尾且不再计入可疑角标（展示信息保留，归「已忽略」折叠区）
        if ignored_ports.contains(&port) {
            info.category = PortCategory::Ignored;
            info.score = 0;
        }
        results.push(info);
    }

    // 5. 排序：score 降序 → 启动早的在前（遗留越久越可疑）→ 端口号升序
    results.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| opt_str_asc(&a.started_at, &b.started_at))
            .then_with(|| a.port.cmp(&b.port))
    });
    results
}

/// 过滤 LISTEN 并按端口号聚合。
fn aggregate_listeners(sockets: &[SocketInfo]) -> HashMap<u16, PortAgg> {
    let mut agg: HashMap<u16, PortAgg> = HashMap::new();
    for si in sockets {
        let ProtocolSocketInfo::Tcp(tcp) = &si.protocol_socket_info else {
            continue;
        };
        if tcp.state != TcpState::Listen {
            continue;
        }
        let entry = agg.entry(tcp.local_port).or_default();
        let (addr_str, unspecified) = format_listen_addr(tcp.local_addr, tcp.local_port);
        if !entry.addresses.contains(&addr_str) {
            entry.addresses.push(addr_str);
        }
        entry.listen_all |= unspecified;
        if entry.owner_pid.is_none() {
            entry.owner_pid = si.associated_pids.first().copied();
        }
    }
    agg
}

/// 组装单个端口的信息 + 分类评分（模块核心逻辑）。
///
/// 分类优先级见模块顶部表格；系统/环境服务档跳过展示信息提取
/// （svchost 条目数量大且命令行无辨识价值）。
#[allow(clippy::too_many_arguments)]
fn build_port_info(
    port: u16,
    a: PortAgg,
    system: &System,
    expected_index: &HashMap<u16, &ProjectPortRef>,
    registry_ids: &HashSet<i64>,
    ide_pids: &HashSet<u32>,
    parent_map: &HashMap<u32, Vec<u32>>,
    self_pid: Option<Pid>,
    now_secs: u64,
) -> ListeningPortInfo {
    let mut info = ListeningPortInfo {
        port,
        addresses: a.addresses,
        listen_all_interfaces: a.listen_all,
        owner_pid: a.owner_pid,
        process_name: String::new(),
        exe: String::new(),
        display_title: String::new(),
        cmdline_summary: String::new(),
        project_path: String::new(),
        started_at: None,
        memory_bytes: 0,
        category: PortCategory::System,
        score: 0,
        orphan: false,
        random_port: false,
        matched_project_name: None,
        matched_project_managed: false,
        tree_pid_list: Vec::new(),
    };

    // ===== owner 不可得（内核监听，如 http.sys）→ System =====
    let Some(pid) = a.owner_pid else {
        return info;
    };
    info.owner_pid = Some(pid);

    // ===== 占用者进程已消失（扫描间隙竞态）→ System =====
    let Some(proc) = system.process(Pid::from_u32(pid)) else {
        return info;
    };

    let name = proc.name().to_string_lossy().into_owned();
    let name_lower = name.to_ascii_lowercase();
    let exe = proc
        .exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let exe_lower = exe.to_ascii_lowercase();
    info.process_name = name.clone();
    info.exe = exe.clone();
    info.memory_bytes = proc.memory();
    info.started_at = proc.start_time().try_into().ok().and_then(unix_to_iso_string);
    info.tree_pid_list = vec![pid];

    // ===== 本应用自身 / 系统进程 → System =====
    if self_pid == Some(Pid::from_u32(pid)) || is_system_process(&name_lower, &exe_lower) {
        return info;
    }

    // ===== 环境服务进程名（mysqld/postgres/redis…）→ EnvService =====
    if is_env_service_process(&name_lower, &exe_lower) {
        info.category = PortCategory::EnvService;
        info.score = 10;
        return info;
    }

    // ===== expected_ports 命中 → KnownProject / KnownProjectExternal =====
    // 用户明确要求：已配置项目的端口不显眼（不管谁启动）。
    if let Some(proj) = expected_index.get(&port) {
        let managed = registry_ids.contains(&proj.project_id)
            || proj
                .last_pid
                .map(|lp| system.process(Pid::from_u32(lp)).is_some())
                .unwrap_or(false);
        info.category = if managed {
            PortCategory::KnownProject
        } else {
            PortCategory::KnownProjectExternal
        };
        info.score = if managed { 15 } else { 20 };
        info.matched_project_name = Some(proj.project_name.clone());
        info.matched_project_managed = managed;
        fill_display_info(&mut info, proc, pid, system, ide_pids, parent_map);
        return info;
    }

    // ===== 知名环境服务端口（兜底：java 跑的 Kafka/ES、docker 转发等）→ EnvService =====
    if ENV_SERVICE_PORTS.contains(&port) {
        info.category = PortCategory::EnvService;
        info.score = 10;
        fill_display_info(&mut info, proc, pid, system, ide_pids, parent_map);
        return info;
    }

    // ===== dev 运行时 → Dev / Suspicious =====
    if is_dev_runtime(&name_lower, &exe_lower) {
        let orphan = is_orphan(pid, proc.parent().map(|p| p.as_u32()), ide_pids, system);
        let random_port = !KNOWN_DEV_PORTS.contains(&port);
        info.orphan = orphan;
        info.random_port = random_port;

        if !orphan && !random_port {
            // 知名 dev 端口 + IDE 父链 = 正常开发中
            info.category = PortCategory::Dev;
            info.score = 50;
        } else {
            // 可疑：孤立（AI CLI/终端遗留）或随机高位端口（测试框架）
            let age_secs = now_secs.saturating_sub(proc.start_time());
            info.category = PortCategory::Suspicious;
            info.score = suspicious_score(orphan, random_port, a.listen_all, age_secs);
        }
        fill_display_info(&mut info, proc, pid, system, ide_pids, parent_map);
        return info;
    }

    // ===== 其他非系统进程 → Other =====
    info.category = PortCategory::Other;
    info.score = 30;
    fill_display_info(&mut info, proc, pid, system, ide_pids, parent_map);
    info
}

/// 填充展示信息（display_title / summary / project_path / 整树 PID）。
///
/// 复用 dev_scan 提取逻辑，对非 java/node 进程优雅退化到进程名。
fn fill_display_info(
    info: &mut ListeningPortInfo,
    proc: &sysinfo::Process,
    pid: u32,
    system: &System,
    ide_pids: &HashSet<u32>,
    parent_map: &HashMap<u32, Vec<u32>>,
) {
    let name = proc.name().to_string_lossy().into_owned();
    let exe = proc
        .exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let cmd: Vec<std::ffi::OsString> = proc.cmd().to_vec();
    let ppid = proc.parent().map(|p| p.as_u32());

    // java 的 main_script 提取逻辑不同，其余运行时按 node 的位置参数规则退化
    let kind = if name.to_ascii_lowercase().starts_with("java") {
        "java"
    } else {
        "node"
    };
    let cwd = proc.cwd().map(|p| p.to_string_lossy().into_owned());
    let dev_cat = classify(pid, &name, &exe, &cmd, ppid, ide_pids, system);
    let main_script = extract_main_script(&cmd, kind, &name);
    info.display_title =
        extract_display_title(&name, &exe, &cmd, dev_cat, &main_script);
    info.cmdline_summary = extract_cmdline_summary(&cmd, kind);
    info.project_path =
        extract_project_path(&cmd, &exe, dev_cat, cwd.as_deref());

    // 整树 PID（前端原样回传 kill_dev_processes 杀整树）
    let mut tree = vec![pid];
    tree.extend(collect_descendants_with_map(pid, parent_map));
    info.tree_pid_list = tree;
}

// ===== 识别辅助 =====

/// 系统进程判定：知名系统进程名，或 exe 位于任意盘符根下的 Windows 目录。
fn is_system_process(name_lower: &str, exe_lower: &str) -> bool {
    SYSTEM_PROCESS_NAMES.contains(&name_lower)
        // 路径段中不允许冒号，":\windows\" 只匹配盘符根下的 Windows 目录，
        // 不会误伤 d:\proj\windows\app.exe 这类用户目录
        || exe_lower.contains(":\\windows\\")
}

/// 环境服务进程判定（按进程名/exe 路径特征）。
fn is_env_service_process(name_lower: &str, exe_lower: &str) -> bool {
    ENV_SERVICE_NAMES.contains(&name_lower)
        || exe_lower.contains("\\docker\\")
        || exe_lower.contains("\\postgres\\")
        || exe_lower.contains("\\mysql\\")
}

/// dev 运行时判定：dev_scan 的 java/node 系 + python/go/dotnet 等。
fn is_dev_runtime(name_lower: &str, exe_lower: &str) -> bool {
    is_dev_process(name_lower, exe_lower)
        || DEV_RUNTIME_EXTRA_NAMES.contains(&name_lower)
        || DEV_RUNTIME_EXE_SUFFIXES.iter().any(|s| exe_lower.ends_with(s))
}

/// suspicious 分数（纯函数，供单测）。
fn suspicious_score(orphan: bool, random_port: bool, exposed: bool, age_secs: u64) -> u32 {
    let mut score = SUSPICIOUS_BASE;
    if orphan {
        score += BONUS_ORPHAN;
    }
    if random_port {
        score += BONUS_RANDOM_PORT;
    }
    if exposed {
        score += BONUS_EXPOSED;
    }
    if age_secs > BONUS_LONG_LIVED_SECS {
        score += BONUS_LONG_LIVED;
    }
    score
}

/// 监听地址格式化：v4 原样，v6 加方括号；返回 (展示串, 是否监听全部网卡)。
fn format_listen_addr(addr: IpAddr, port: u16) -> (String, bool) {
    let unspecified = addr.is_unspecified();
    let s = match addr {
        IpAddr::V4(v4) => format!("{v4}:{port}"),
        IpAddr::V6(v6) => format!("[{v6}]:{port}"),
    };
    (s, unspecified)
}

/// Option<String> 升序比较，None 排最后（启动时间未知视为最新）。
fn opt_str_asc(a: &Option<String>, b: &Option<String>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};

    /// 刷新测试自身进程所需的 ProcessRefreshKind（与命令层 port_refresh_kind 一致）。
    fn self_refresh_kind() -> ProcessRefreshKind {
        ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_exe(UpdateKind::Always)
            .with_memory()
    }

    fn tcp_listen(local_addr: IpAddr, local_port: u16, pids: Vec<u32>) -> SocketInfo {
        SocketInfo {
            protocol_socket_info: ProtocolSocketInfo::Tcp(netstat2::TcpSocketInfo {
                local_addr,
                local_port,
                remote_addr: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                remote_port: 0,
                state: TcpState::Listen,
            }),
            associated_pids: pids,
        }
    }

    #[test]
    fn system_process_detection() {
        assert!(is_system_process("svchost.exe", ""));
        assert!(is_system_process("anything.exe", "c:\\windows\\system32\\foo.exe"));
        assert!(!is_system_process("node.exe", "c:\\nodejs\\node.exe"));
        // 任意盘符根下的 Windows 目录都算系统进程
        assert!(is_system_process("foo.exe", "d:\\windows\\foo.exe"));
        // 路径中间包含 windows 段的用户目录不算（历史回归：曾按 contains("\windows\") 误判）
        assert!(!is_system_process("foo.exe", "d:\\proj\\windows\\foo.exe"));
    }

    #[test]
    fn env_service_detection() {
        assert!(is_env_service_process("mysqld.exe", ""));
        assert!(is_env_service_process("postgres.exe", ""));
        assert!(is_env_service_process(
            "com.docker.backend.exe",
            "c:\\program files\\docker\\docker\\resources\\com.docker.backend.exe"
        ));
        assert!(!is_env_service_process("node.exe", ""));
    }

    #[test]
    fn dev_runtime_covers_python_and_node() {
        assert!(is_dev_runtime("python.exe", "c:\\python\\python.exe"));
        assert!(is_dev_runtime("node.exe", "c:\\nodejs\\node.exe"));
        assert!(is_dev_runtime("java.exe", ""));
        assert!(is_dev_runtime("foo.exe", "c:\\go\\bin\\go.exe"));
        assert!(!is_dev_runtime("telegram.exe", "c:\\app\\telegram.exe"));
    }

    #[test]
    fn suspicious_score_components() {
        // 仅基础分
        assert_eq!(suspicious_score(false, false, false, 0), SUSPICIOUS_BASE);
        // 孤立 + 随机端口
        assert_eq!(
            suspicious_score(true, true, false, 0),
            SUSPICIOUS_BASE + BONUS_ORPHAN + BONUS_RANDOM_PORT
        );
        // 全部加分
        assert_eq!(
            suspicious_score(true, true, true, BONUS_LONG_LIVED_SECS + 1),
            SUSPICIOUS_BASE + BONUS_ORPHAN + BONUS_RANDOM_PORT + BONUS_EXPOSED + BONUS_LONG_LIVED
        );
        // 存活恰好 1 小时不加分（严格大于）
        assert_eq!(
            suspicious_score(false, false, false, BONUS_LONG_LIVED_SECS),
            SUSPICIOUS_BASE
        );
    }

    #[test]
    fn dev_and_env_port_tables_no_overlap() {
        for p in KNOWN_DEV_PORTS {
            assert!(
                !ENV_SERVICE_PORTS.contains(p),
                "端口 {p} 同时在 dev 表与环境服务表中，判定会有歧义"
            );
        }
    }

    #[test]
    fn format_addr_v4_v6_unspecified() {
        let (s, all) = format_listen_addr(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);
        assert_eq!(s, "127.0.0.1:3000");
        assert!(!all);

        let (s, all) = format_listen_addr(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
        assert_eq!(s, "0.0.0.0:8080");
        assert!(all);

        let (s, all) = format_listen_addr(IpAddr::V6(Ipv6Addr::LOCALHOST), 5173);
        assert_eq!(s, "[::1]:5173");
        assert!(!all);

        let (s, all) = format_listen_addr(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 5173);
        assert_eq!(s, "[::]:5173");
        assert!(all);
    }

    #[test]
    fn aggregate_merges_v4_v6_same_port() {
        let sockets = vec![
            tcp_listen(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 5173, vec![100]),
            tcp_listen(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 5173, vec![100]),
        ];
        let agg = aggregate_listeners(&sockets);
        assert_eq!(agg.len(), 1);
        let a = &agg[&5173];
        assert_eq!(a.addresses.len(), 2);
        assert!(a.listen_all);
        assert_eq!(a.owner_pid, Some(100));
    }

    #[test]
    fn aggregate_takes_first_owner_pid() {
        let sockets = vec![tcp_listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080, vec![200, 300])];
        let agg = aggregate_listeners(&sockets);
        assert_eq!(agg[&8080].owner_pid, Some(200));
    }

    /// 空进程表 + 无 owner 的 LISTEN（内核监听场景）→ 全部归 System，按端口升序。
    #[test]
    fn scan_kernel_listeners_as_system() {
        let system = System::new();
        let sockets = vec![
            tcp_listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 80, vec![]),
            tcp_listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 135, vec![]),
        ];
        let results = scan_listening_ports(&system, &sockets, &[], &HashSet::new(), &HashSet::new(), None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].port, 80); // 同 score 按端口升序
        assert_eq!(results[0].category, PortCategory::System);
        assert_eq!(results[0].score, 0);
        assert_eq!(results[0].owner_pid, None);
    }

    /// owner PID 不在进程表（竞态消失）→ 归 System 容错。
    #[test]
    fn scan_missing_owner_process_falls_back_to_system() {
        let system = System::new();
        let sockets = vec![tcp_listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 9999, vec![12345])];
        let results = scan_listening_ports(&system, &sockets, &[], &HashSet::new(), &HashSet::new(), None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].category, PortCategory::System);
        assert_eq!(results[0].owner_pid, Some(12345));
    }

    /// expected_ports 命中分类：注入测试自身真实进程作为占用者
    /// （sysinfo 无法伪造进程表，只能刷新真实进程）。
    /// - last_pid 不存活且不在 registry → KnownProjectExternal
    /// - 在 registry → KnownProject
    #[test]
    fn scan_expected_port_match() {
        // 刷新测试自身进程，owner 指向它（非系统/环境/dev 进程，走 expected 分支）
        let mut system = System::new();
        let pid = sysinfo::get_current_pid().expect("获取测试进程 PID");
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            self_refresh_kind(),
        );
        let sockets = vec![tcp_listen(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            8080,
            vec![pid.as_u32()],
        )];
        let projects = vec![ProjectPortRef {
            project_id: 1,
            project_name: "demo".into(),
            ports: vec![8080],
            last_pid: Some(999), // 不在进程表 → 不存活 → external
        }];

        // 未托管：KnownProjectExternal
        let results = scan_listening_ports(
            &system,
            &sockets,
            &projects,
            &HashSet::new(),
            &HashSet::new(),
            None,
        );
        assert_eq!(results[0].category, PortCategory::KnownProjectExternal);
        assert_eq!(results[0].score, 20);
        assert_eq!(results[0].matched_project_name.as_deref(), Some("demo"));
        assert!(!results[0].matched_project_managed);

        // in_registry → KnownProject
        let mut ids = HashSet::new();
        ids.insert(1i64);
        let results = scan_listening_ports(
            &system,
            &sockets,
            &projects,
            &ids,
            &HashSet::new(),
            None,
        );
        assert_eq!(results[0].category, PortCategory::KnownProject);
        assert_eq!(results[0].score, 15);
        assert!(results[0].matched_project_managed);
    }

    /// 知名环境服务端口（3306）+ 真实占用者进程 → EnvService（端口表兜底）。
    #[test]
    fn scan_env_service_port_with_owner() {
        let mut system = System::new();
        let pid = sysinfo::get_current_pid().expect("获取测试进程 PID");
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            self_refresh_kind(),
        );
        let sockets = vec![tcp_listen(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            3306,
            vec![pid.as_u32()],
        )];
        let results = scan_listening_ports(&system, &sockets, &[], &HashSet::new(), &HashSet::new(), None);
        assert_eq!(results[0].category, PortCategory::EnvService);
        assert_eq!(results[0].score, 10);
    }

    /// 未知进程（测试自身，非系统/环境/dev）+ 未知端口 → Other。
    #[test]
    fn scan_unknown_process_as_other() {
        let mut system = System::new();
        let pid = sysinfo::get_current_pid().expect("获取测试进程 PID");
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            self_refresh_kind(),
        );
        let sockets = vec![tcp_listen(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            55555,
            vec![pid.as_u32()],
        )];
        let results = scan_listening_ports(&system, &sockets, &[], &HashSet::new(), &HashSet::new(), None);
        assert_eq!(results[0].category, PortCategory::Other);
        assert_eq!(results[0].score, 30);
    }

    /// owner 不可得时优先归 System（内核判定优先于环境服务端口表）。
    #[test]
    fn scan_kernel_listener_on_service_port_still_system() {
        let system = System::new();
        let sockets = vec![tcp_listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 3306, vec![])];
        let results = scan_listening_ports(&system, &sockets, &[], &HashSet::new(), &HashSet::new(), None);
        assert_eq!(results[0].category, PortCategory::System);
    }

    /// 忽略名单覆盖：命中 port_ignore 的端口无论推断为何分类
    /// 一律 → Ignored / score 0（用户显式意志优先于所有推断），
    /// 沉到排序末尾、不再计入可疑角标。
    #[test]
    fn scan_ignored_port_overrides_category() {
        let mut system = System::new();
        let pid = sysinfo::get_current_pid().expect("获取测试进程 PID");
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            self_refresh_kind(),
        );
        // 两个同分类端口（测试自身进程 → Other），仅 55555 在忽略名单
        let sockets = vec![
            tcp_listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 55555, vec![pid.as_u32()]),
            tcp_listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 55556, vec![pid.as_u32()]),
        ];
        let ignored: HashSet<u16> = HashSet::from([55555]);

        let results = scan_listening_ports(
            &system,
            &sockets,
            &[],
            &HashSet::new(),
            &ignored,
            None,
        );
        // 未忽略端口保持原分类在前；被忽略端口覆盖为 Ignored 并沉底
        assert_eq!(results[0].port, 55556);
        assert_eq!(results[0].category, PortCategory::Other);
        assert_eq!(results[1].port, 55555);
        assert_eq!(results[1].category, PortCategory::Ignored);
        assert_eq!(results[1].score, 0);
    }
}
