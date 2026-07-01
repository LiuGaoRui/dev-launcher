//! 全系统开发进程扫描 + 智能分类。
//!
//! 与 `monitor.rs` 的区别：monitor 只看「本工具托管的项目进程」（registry + DB last_pid）；
//! 本模块扫描整机所有 java/node 相关进程——含 IDEA/VSCode 启动的、孤立的守护进程、
//! 构建 daemon、dev server——用于内存清理器的「发现 + 推荐清理」场景。
//!
//! 智能分类策略（详见各 `classify_*` 函数）：
//! - IDE 自身进程受保护，永不推荐清理
//! - 构建 daemon（Gradle/Maven）长时间挂着吃内存 → 阈值以上推荐
//! - dev server（Vite/webpack/SpringBoot）内存膨胀 → 阈值以上推荐
//! - 孤立进程（父祖链无 IDE）→ 直接推荐
//!
//! 复用：`collect_tree`（进程树聚合内存/CPU）、持久化 `sysinfo::System`。

use std::collections::HashSet;

use serde::Serialize;
use sysinfo::{Pid, System};

use crate::process::tree::{aggregate_tree, build_parent_map, collect_descendants_with_map};

// ===== 序列化结构（对齐前端 types/cleaner.ts，serde snake_case） =====

/// 开发进程智能分类标签。
///
/// 语义见模块顶部说明；`IdeMain` 受保护不可清理。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DevProcCategory {
    /// IDE 自身进程（IDEA/VSCode），受保护不可清理
    IdeMain,
    /// 构建 daemon（Gradle/Maven daemon、编译器常驻进程）
    BuildDaemon,
    /// dev server（Vite/webpack/SpringBoot dev 模式）
    DevServer,
    /// 孤立进程（父祖链无 IDE，通常是关闭项目后未释放的残留）
    Orphan,
    /// 文件监视器（tsc/esbuild --watch、nodemon）
    Watcher,
    /// 其他 java/node 开发进程（未匹配以上模式）
    Other,
}

/// 单个开发进程的扫描结果。
#[derive(Debug, Clone, Serialize)]
pub struct DevProcInfo {
    pub pid: u32,
    /// 父进程 PID（None 表示无父进程或不可达）
    pub ppid: Option<u32>,
    /// 进程大类：java | node
    pub kind: String,
    /// 进程名（如 java.exe、node.exe）
    pub name: String,
    /// 可执行文件路径
    pub exe: String,
    /// 完整命令行（空格连接，前端展示用）
    pub cmdline: String,
    /// 从命令行提取的项目/模块提示（如 artifactId、package.json name）
    pub cmdline_hint: String,
    /// 用户友好的展示名（如「IntelliJ IDEA」「VSCode 扩展宿主」「vite dev」），
    /// 比单纯的进程名（code.exe / java.exe）更易辨识，用于前端列表主标题。
    pub display_title: String,
    /// 关联的项目/工作目录路径（从命令行 -jar/cwd/IDE 工作区等提取），可能为空。
    /// 帮助用户判断「这是哪个项目的进程」，是「是否该清理」的关键决策信息。
    pub project_path: String,
    /// 进程工作目录（sysinfo Process::cwd()），可能为 None（权限不足或已退出）。
    /// 比 project_path（从命令行推断）更权威，是「进程在哪个目录跑的」直接证据。
    pub cwd: Option<String>,
    /// 命令行语义化摘要：提取正在执行的主脚本 / main class / npm script 等，
    /// 比 exe 路径更能说明「进程在做什么」。如 `vite.js --port 5173`、`com.example.App`。
    pub cmdline_summary: String,
    /// 正在执行的主程序名（最精简辨识），如 `server.js`、`Application`、`demo.jar`。
    /// 用于 display_title 兜底和搜索匹配。
    pub main_script: String,
    /// 本进程 RSS（字节）
    pub memory_bytes: u64,
    /// 整个进程树内存（含子进程，字节）
    pub tree_memory_bytes: u64,
    /// 整树 CPU 占用百分比（归一化到 0-100）
    pub cpu_percent: f32,
    /// 启动时间（ISO 字符串，可能为空）
    pub started_at: Option<String>,
    /// 智能分类
    pub category: DevProcCategory,
    /// 整树 PID 列表（前端清理时原样回传）
    pub tree_pid_list: Vec<u32>,
    /// 是否推荐清理（综合分类 + 内存阈值判定）
    pub recommended: bool,
}

// ===== 阈值常量（内存，字节） =====
//
// 取值偏激进：开发机的痛点就是「关项目后内存不释放」，
// 故 daemon / dev server 达到中高水位即推荐清理。
const BUILD_DAEMON_RECOMMEND_BYTES: u64 = 512 * 1024 * 1024; // 512 MB
const DEV_SERVER_RECOMMEND_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB

/// 命令行展示截断长度（过长的 Gradle/Java 命令行截断避免前端渲染压力）
const CMDLINE_DISPLAY_MAX: usize = 400;

/// 扫描全系统开发进程，返回分类后的列表（按 tree_memory_bytes 降序）。
///
/// `system` 应已 `refresh_processes(All)`（由调用方在持锁时刷新）。
/// `self_pid` 为本应用自身 PID，排除避免误杀自己。
pub fn scan_dev_processes(system: &System, self_pid: Option<Pid>) -> Vec<DevProcInfo> {
    let num_cpus = system.cpus().len().max(1) as f32;

    // 第一遍：筛选出开发进程的根候选（按进程名/exe）
    // 同时建立「IDE 进程 PID 集合」用于孤立判定
    let mut candidates: Vec<Pid> = Vec::new();
    let mut ide_pids: HashSet<u32> = HashSet::new();

    for (pid, proc) in system.processes() {
        let pid_u32 = pid.as_u32();
        // 排除自身
        if self_pid == Some(*pid) {
            continue;
        }
        let name = proc.name().to_string_lossy().to_ascii_lowercase();
        let exe = proc
            .exe()
            .map(|p| p.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        if is_ide_process(&name, &exe, proc) {
            ide_pids.insert(pid_u32);
            // IDE 进程本身也作为候选（展示 + 标记受保护），但不一定都展示
            candidates.push(*pid);
        } else if is_dev_process(&name, &exe) {
            candidates.push(*pid);
        }
    }

    // 第二遍：构建 parent→children 索引一次，避免每个候选重复扫描全进程表
    let parent_map = build_parent_map(system);

    let mut results: Vec<DevProcInfo> = Vec::with_capacity(candidates.len());
    for pid in &candidates {
        let pid_u32 = pid.as_u32();
        let Some(proc) = system.process(*pid) else {
            continue;
        };

        // 整树 PID（用预建索引） + 内存/CPU 聚合
        let mut tree_pids = vec![pid_u32];
        tree_pids.extend(collect_descendants_with_map(pid_u32, &parent_map));
        let (raw_cpu, tree_mem) = aggregate_tree(&tree_pids, system);
        let cpu_percent = raw_cpu / num_cpus;
        let self_mem = proc.memory();

        let name = proc.name().to_string_lossy().into_owned();
        let exe = proc
            .exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let cmdline_vec = proc.cmd();
        let cmdline_full = join_cmdline(cmdline_vec);
        let cmdline_display = truncate_cmdline(&cmdline_full);

        let ppid = proc.parent().map(|p| p.as_u32());
        let started_at = proc
            .start_time()
            .try_into()
            .ok()
            .and_then(unix_to_iso_string);

        let kind = if is_java_process(&name) {
            "java"
        } else {
            "node"
        };

        // 先分类（display_title / project_path 依赖 category 做针对性提取）
        let category = classify(
            pid_u32,
            &name,
            &exe,
            cmdline_vec,
            ppid,
            &ide_pids,
            system,
        );

        // 再基于分类提取展示信息
        let cmdline_hint = extract_hint(cmdline_vec, &exe, &name);
        let main_script = extract_main_script(cmdline_vec, &kind, &name);
        let cmdline_summary = extract_cmdline_summary(cmdline_vec, &kind);
        let cwd = proc
            .cwd()
            .map(|p| p.to_string_lossy().into_owned());
        let display_title =
            extract_display_title(&name, &exe, cmdline_vec, category, &main_script);
        let project_path = extract_project_path(cmdline_vec, &exe, category, cwd.as_deref());

        let recommended = is_recommended(category, tree_mem, pid_u32, self_pid);

        results.push(DevProcInfo {
            pid: pid_u32,
            ppid,
            kind: kind.into(),
            name,
            exe,
            cmdline: cmdline_display,
            cmdline_hint,
            display_title,
            project_path,
            cwd,
            cmdline_summary,
            main_script,
            memory_bytes: self_mem,
            tree_memory_bytes: tree_mem,
            cpu_percent,
            started_at,
            category,
            tree_pid_list: tree_pids,
            recommended,
        });
    }

    // 降序：内存大的在前（清理优先级高）
    results.sort_by(|a, b| b.tree_memory_bytes.cmp(&a.tree_memory_bytes));
    results
}

// ===== 进程识别 =====

/// 判断是否为开发相关进程（java/javaw/node/npm/yarn/pnpm 等）。
fn is_dev_process(name_lower: &str, exe_lower: &str) -> bool {
    is_java_process(name_lower) || is_node_process(name_lower, exe_lower)
}

/// Java 进程：java.exe / javaw.exe（jvm 运行的服务、Gradle/Maven daemon 都是 java 启动）。
fn is_java_process(name_lower: &str) -> bool {
    name_lower == "java.exe" || name_lower == "javaw.exe" || name_lower == "java" || name_lower == "javaw"
}

/// Node 进程：node.exe / npm.exe / yarn.exe / pnpm.exe / npx.exe。
/// 注意：electron 进程不算（除非命令行含 dev 标志，由 classify 进一步判定）。
fn is_node_process(name_lower: &str, exe_lower: &str) -> bool {
    matches!(
        name_lower,
        "node.exe" | "node" | "npm.exe" | "yarn.exe" | "pnpm.exe" | "npx.exe"
    ) || exe_lower.ends_with("\\node.exe")
        || exe_lower.ends_with("/node")
}

/// 判断是否为 IDE 自身进程（IDEA / VSCode / WebStorm 等的主进程）。
///
/// 这些进程的父祖链根，**不应**被清理。判定依据：进程名或 exe 路径特征。
fn is_ide_process(name_lower: &str, exe_lower: &str, proc: &sysinfo::Process) -> bool {
    // VSCode 主进程：Code.exe 且 exe 路径含 Microsoft VS Code
    if name_lower == "code.exe" || name_lower == "code" {
        return exe_lower.contains("vscode") || exe_lower.contains("code");
    }
    // JetBrains 系列（IDEA/WebStorm/PyCharm 等）：进程名常为 idea64.exe / webstorm64.exe
    // 统一特征：exe 路径含 jetbrains
    if exe_lower.contains("jetbrains") {
        return true;
    }
    // 常见 JetBrains 主程序名
    const JB_NAMES: &[&str] = &[
        "idea.exe", "idea64.exe", "webstorm.exe", "webstorm64.exe",
        "pycharm.exe", "pycharm64.exe", "goland.exe", "goland64.exe",
        "clion.exe", "clion64.exe", "phpstorm.exe", "phpstorm64.exe",
        "rubymine.exe", "rubymine64.exe", "rider.exe", "rider64.exe",
        "studio64.exe", // Android Studio
        "idea", "webstorm",
    ];
    if JB_NAMES.contains(&name_lower) {
        return true;
    }
    // 兜底：命令行含明确 IDE 启动参数（如 -Didea.* / -Dvscode.*）
    // 避免误判普通 java 进程为 IDE
    let cmdline = proc.cmd();
    if cmdline.iter().any(|s| {
        let s = s.to_string_lossy().to_ascii_lowercase();
        s.starts_with("-didea.")
            || s.contains("com.intellij.idea")
            || s.contains("idea.platform.prefix")
    }) {
        return true;
    }
    false
}

// ===== 智能分类 =====

/// 对单个候选进程执行分类判定。
#[allow(clippy::too_many_arguments)]
fn classify(
    pid: u32,
    name: &str,
    exe: &str,
    cmdline: &[std::ffi::OsString],
    ppid: Option<u32>,
    ide_pids: &HashSet<u32>,
    system: &System,
) -> DevProcCategory {
    let name_lower = name.to_ascii_lowercase();
    let exe_lower = exe.to_ascii_lowercase();

    // 1. IDE 自身进程 → 受保护
    // classify 由 scan_dev_processes 在确认进程存在后调用，此处再取一次避免重复查；
    // 进程恰好在此间隙消失则按「非 IDE」处理，后续走孤立判定兜底。
    let proc_ref = system.process(Pid::from_u32(pid));
    if proc_ref.map_or(false, |p| is_ide_process(&name_lower, &exe_lower, p)) {
        return DevProcCategory::IdeMain;
    }

    let cmdline_str = join_cmdline(cmdline);
    let cmdline_lower = cmdline_str.to_ascii_lowercase();

    // 2. 构建 daemon
    if is_build_daemon(&name_lower, &cmdline_lower) {
        return DevProcCategory::BuildDaemon;
    }

    // 3. dev server
    if is_dev_server(&cmdline_lower) {
        return DevProcCategory::DevServer;
    }

    // 4. 文件监视器（watch 模式）
    if is_watcher(&cmdline_lower) {
        return DevProcCategory::Watcher;
    }

    // 5. 孤立判定：父祖链根不是 IDE 进程
    if is_orphan(pid, ppid, ide_pids, system) {
        return DevProcCategory::Orphan;
    }

    DevProcCategory::Other
}

/// 构建 daemon 特征：
/// - Gradle daemon：命令行含 `org.gradle.launcher.daemon`
/// - Maven daemon：`-Dmaven.*` 或 `mvn` 守护
/// - Kotlin/Scala 编译器 daemon：`-Dkotlin.*` / `scala.tools.*`
fn is_build_daemon(name_lower: &str, cmdline_lower: &str) -> bool {
    // Gradle daemon（最常见，java 进程）
    if cmdline_lower.contains("org.gradle.launcher.daemon")
        || cmdline_lower.contains("gradle.daemon")
    {
        return true;
    }
    // Maven
    if cmdline_lower.contains("org.codehaus.plexus.classworlds.launcher")
        || cmdline_lower.contains("org.apache.maven")
    {
        return true;
    }
    // Kotlin daemon
    if cmdline_lower.contains("org.jetbrains.kotlin.compiler")
        || cmdline_lower.contains("-dkotlin.daemon")
    {
        return true;
    }
    // Scala / SBT
    if cmdline_lower.contains("scala.tools.nsc") || cmdline_lower.contains("sbt") {
        return true;
    }
    // Maven/Gradle 直接命令
    if name_lower == "mvn" || name_lower == "mvn.exe" || name_lower == "gradle" || name_lower == "gradle.exe" {
        return true;
    }
    // Tomcat/Undertow 独立启动（非 dev server 模式）
    if cmdline_lower.contains("org.apache.catalina.startup") && !cmdline_lower.contains("spring-boot") {
        return true;
    }
    false
}

/// dev server 特征：
/// - Node 侧：vite / webpack serve / next dev / react-scripts start / nodemon / vue-cli-service serve
/// - Java 侧：spring-boot:run / -Dspring-boot.run
fn is_dev_server(cmdline_lower: &str) -> bool {
    // Node dev server
    const NODE_DEV_MARKERS: &[&str] = &[
        "vite",
        "webpack serve",
        "webpack-dev-server",
        "next dev",
        "react-scripts start",
        "vue-cli-service serve",
        "nuxt dev",
        "astro dev",
        "svelte-kit dev",
        "@angular-devkit",
        "ng serve",
    ];
    if NODE_DEV_MARKERS.iter().any(|m| cmdline_lower.contains(m)) {
        return true;
    }
    // Java spring-boot dev
    if cmdline_lower.contains("spring-boot:run")
        || cmdline_lower.contains("spring-boot.run")
        || cmdline_lower.contains("org.springframework.boot.loader")
    {
        return true;
    }
    false
}

/// 文件监视器特征：--watch / --watch-poll / nodemon
fn is_watcher(cmdline_lower: &str) -> bool {
    cmdline_lower.contains("--watch")
        || cmdline_lower.contains("tsc --watch")
        || cmdline_lower.contains("esbuild --watch")
        || cmdline_lower.contains("nodemon")
        || cmdline_lower.contains("chokidar")
}

/// 孤立进程判定：向上追溯父进程链，若根祖不是 IDE 进程则视为孤立。
///
/// 孤立进程的典型场景：用户在 IDE 里启动了 dev server / 构建进程，
/// 关闭项目后这些子进程未随 IDE 退出而退出（无 kill_on_drop），残留在内存中。
fn is_orphan(
    _pid: u32,
    ppid: Option<u32>,
    ide_pids: &HashSet<u32>,
    system: &System,
) -> bool {
    // 防止循环引用（理论上进程树无环，但加保险）
    const MAX_DEPTH: u32 = 32;

    let mut current = ppid;
    let mut depth = 0u32;
    while let Some(parent) = current {
        if depth >= MAX_DEPTH {
            break;
        }
        depth += 1;

        if ide_pids.contains(&parent) {
            // 父祖链中存在 IDE 进程 → 非孤立（由 IDE 派生）
            return false;
        }
        // 继续向上
        current = system
            .process(Pid::from_u32(parent))
            .and_then(|p| p.parent())
            .map(|p| p.as_u32());
    }
    // 递归到根仍未遇到 IDE → 孤立
    true
}

// ===== 推荐清理判定 =====

/// 综合分类 + 内存阈值，决定是否推荐清理。
///
/// 策略（偏激进，贴合「关项目后内存不释放」痛点）：
/// - `Orphan` → 一律推荐
/// - `BuildDaemon` 且整树内存 ≥ 512MB → 推荐
/// - `DevServer` 且整树内存 ≥ 1GB → 推荐
/// - `IdeMain` → 永不推荐（受保护）
/// - `Watcher` / `Other` → 不主动推荐（用户可手动选）
fn is_recommended(category: DevProcCategory, tree_mem: u64, pid: u32, self_pid: Option<Pid>) -> bool {
    // 永不推荐自身
    if self_pid == Some(Pid::from_u32(pid)) {
        return false;
    }
    match category {
        DevProcCategory::IdeMain => false,
        DevProcCategory::Orphan => true,
        DevProcCategory::BuildDaemon => tree_mem >= BUILD_DAEMON_RECOMMEND_BYTES,
        DevProcCategory::DevServer => tree_mem >= DEV_SERVER_RECOMMEND_BYTES,
        DevProcCategory::Watcher => false,
        DevProcCategory::Other => false,
    }
}

// ===== 辅助 =====

/// 把命令行参数向量连接为单字符串（用于分类匹配与展示）。
fn join_cmdline(cmd: &[std::ffi::OsString]) -> String {
    let parts: Vec<String> = cmd
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    parts.join(" ")
}

/// 截断命令行到展示长度上限。
fn truncate_cmdline(s: &str) -> String {
    if s.chars().count() <= CMDLINE_DISPLAY_MAX {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(CMDLINE_DISPLAY_MAX).collect();
        format!("{truncated}…")
    }
}

/// JetBrains 产品名映射（exe 路径特征 → 产品中文名）。
/// 用于 IDE 进程的 display_title，让用户一眼看出是 IDEA / WebStorm 还是其他产品。
const JB_PRODUCTS: &[(&str, &str)] = &[
    ("idea", "IntelliJ IDEA"),
    ("idea64", "IntelliJ IDEA"),
    ("webstorm", "WebStorm"),
    ("webstorm64", "WebStorm"),
    ("pycharm", "PyCharm"),
    ("pycharm64", "PyCharm"),
    ("goland", "GoLand"),
    ("goland64", "GoLand"),
    ("clion", "CLion"),
    ("clion64", "CLion"),
    ("phpstorm", "PhpStorm"),
    ("phpstorm64", "PhpStorm"),
    ("rubymine", "RubyMine"),
    ("rubymine64", "RubyMine"),
    ("rider", "Rider"),
    ("rider64", "Rider"),
    ("studio64", "Android Studio"), // Android Studio
    ("studio", "Android Studio"),
    ("fleet", "JetBrains Fleet"),
    ("datagrip", "DataGrip"),
    ("datagrip64", "DataGrip"),
];

/// 提取用户友好的展示名（display_title）。
///
/// 比 cmdline_hint（项目模块提示）更聚焦「这是什么程序」：
/// - IDE：识别具体产品（IntelliJ IDEA / VSCode 主进程 / 扩展宿主 / 辅助进程）
/// - dev server：识别框架（vite / webpack-dev-server / SpringBoot）
/// - 构建 daemon：识别构建工具（Gradle Daemon / Maven）
/// - 其他：退化到 cmdline_hint 或进程名
fn extract_display_title(
    name: &str,
    exe: &str,
    cmd: &[std::ffi::OsString],
    category: DevProcCategory,
    main_script: &str,
) -> String {
    let name_lower = name.to_ascii_lowercase();
    let exe_lower = exe.to_ascii_lowercase();
    let cmdline_lower = join_cmdline(cmd).to_ascii_lowercase();

    // ===== IDE 进程：识别具体产品 + 角色 =====
    if category == DevProcCategory::IdeMain {
        // VSCode 系：区分主进程 / 扩展宿主 / 渲染进程等
        if name_lower == "code.exe" || name_lower == "code" || exe_lower.contains("vscode") || exe_lower.contains("\\code.exe") {
            if let Some(role) = identify_vscode_role(&cmdline_lower) {
                return format!("VSCode {role}");
            }
            return "VSCode".to_string();
        }
        // JetBrains 系：从 exe 文件名匹配产品
        let exe_file = exe_lower.split(['/', '\\']).next_back().unwrap_or(&name_lower);
        let exe_stem = exe_file.trim_end_matches(".exe");
        for (key, product) in JB_PRODUCTS {
            if exe_stem == *key || name_lower.starts_with(key) {
                return (*product).to_string();
            }
        }
        // JetBrains 兜底：命令行含 idea 启动参数
        if cmdline_lower.contains("com.intellij.idea") || cmdline_lower.contains("-didea.") {
            return "IntelliJ IDEA".to_string();
        }
        if exe_lower.contains("jetbrains") {
            return "JetBrains IDE".to_string();
        }
        return "IDE".to_string();
    }

    // ===== 构建 daemon =====
    if category == DevProcCategory::BuildDaemon {
        if cmdline_lower.contains("gradle") {
            return "Gradle Daemon".to_string();
        }
        if cmdline_lower.contains("maven") || cmdline_lower.contains("plexus.classworlds") {
            return "Maven".to_string();
        }
        if cmdline_lower.contains("kotlin") {
            return "Kotlin Daemon".to_string();
        }
        if cmdline_lower.contains("scala") || cmdline_lower.contains("sbt") {
            return "Scala/sbt".to_string();
        }
        if cmdline_lower.contains("catalina") {
            return "Tomcat".to_string();
        }
        return "构建守护进程".to_string();
    }

    // ===== dev server：识别框架 =====
    if category == DevProcCategory::DevServer {
        return identify_dev_server_name(&cmdline_lower);
    }

    // ===== watcher =====
    if category == DevProcCategory::Watcher {
        if cmdline_lower.contains("tsc") {
            return "tsc --watch".to_string();
        }
        if cmdline_lower.contains("nodemon") {
            return "nodemon".to_string();
        }
        if cmdline_lower.contains("esbuild") {
            return "esbuild --watch".to_string();
        }
        return "文件监视器".to_string();
    }

    // ===== orphan / other：优先用 main_script（正在执行什么），再退化 =====
    // 这是对裸 node/java 进程辨识度提升的关键：node server.js → "server.js (node)"
    if !main_script.is_empty() && main_script != name {
        // 运行时类型后缀（java/node），帮助用户一眼看出技术栈
        let kind_label = if is_java_process(name) { "java" } else { "node" };
        return format!("{main_script} ({kind_label})");
    }
    // 兜底：尝试 cmdline_hint
    let hint = extract_hint(cmd, exe, name);
    if !hint.is_empty() && hint != name {
        return hint;
    }
    name.to_string()
}

/// 从 VSCode 命令行参数识别进程角色（主进程 / 扩展宿主 / 渲染进程等）。
fn identify_vscode_role(cmdline_lower: &str) -> Option<String> {
    // VSCode 多进程通过 --type=<role> 区分，role 值取到空格/参数边界
    // （含 hyphen，如 gpu-process）。命令行已转小写，故匹配用小写。
    for token in cmdline_lower.split_whitespace() {
        if let Some(value) = token.strip_prefix("--type=") {
            // value 可能带引号或尾随参数，取到第一个非值字符
            let role = value.trim_matches(|c: char| c == '"' || c == '\'');
            return Some(match role {
                "extensionhost" => "扩展宿主".to_string(),
                "renderer" => "渲染进程".to_string(),
                "gpu-process" => "GPU 进程".to_string(),
                "utility" => "工具进程".to_string(),
                "sharedarraybuffer" => "共享内存进程".to_string(),
                other => format!("子进程({other})"),
            });
        }
    }
    // 无 --type 的通常是主进程
    if cmdline_lower.contains(".code-workspace") || cmdline_lower.contains("--folder-uri") {
        return Some("主进程".to_string());
    }
    None
}

/// 识别 dev server 框架名（用于 display_title）。
fn identify_dev_server_name(cmdline_lower: &str) -> String {
    // 按特征长度降序匹配，避免短串误命中
    const MARKERS: &[(&str, &str)] = &[
        ("react-scripts", "React dev server"),
        ("vue-cli-service", "Vue dev server"),
        ("webpack-dev-server", "webpack dev server"),
        ("webpack serve", "webpack dev server"),
        ("@vitejs", "vite dev server"),
        ("vite", "vite dev server"),
        ("next dev", "Next.js dev server"),
        ("nuxt dev", "Nuxt dev server"),
        ("astro dev", "Astro dev server"),
        ("svelte-kit", "SvelteKit dev server"),
        ("ng serve", "Angular dev server"),
        ("@angular-devkit", "Angular dev server"),
        ("spring-boot:run", "Spring Boot"),
        ("spring-boot.run", "Spring Boot"),
        ("springframework.boot.loader", "Spring Boot"),
    ];
    for (marker, label) in MARKERS {
        if cmdline_lower.contains(marker) {
            return (*label).to_string();
        }
    }
    "dev server".to_string()
}

/// 提取进程正在执行的主程序名（main_script）——最精简的辨识信息。
///
/// 策略按运行时类型：
/// - Node：第一个非选项位置参数（跳过 node.exe、- 开头选项、-e/-r 等带值选项的值），
///   取其文件名（如 `vite.js`、`server.js`）；npm/yarn/pnpm 时取 script 名（如 `dev`）
/// - Java：main class 简名（最后一个含包名的 class 取简名，如 `Application`），
///   或 `-jar xxx.jar` 的 jar 文件名（如 `demo.jar`）
/// - 其他：退化到进程名
fn extract_main_script(cmd: &[std::ffi::OsString], kind: &str, name: &str) -> String {
    // 跳过 exe 本身，从第二个参数开始
    if cmd.len() < 2 {
        return String::new();
    }
    let args: Vec<&std::ffi::OsString> = cmd.iter().skip(1).collect();

    if kind == "java" {
        return extract_java_main_script(&args);
    }
    // node 系
    extract_node_main_script(&args, name)
}

/// 从 Java 命令行参数提取主程序名。
fn extract_java_main_script(args: &[&std::ffi::OsString]) -> String {
    // 优先：-jar xxx.jar → jar 文件名
    if let Some(jar_idx) = args.iter().position(|s| **s == "-jar" || **s == "-jar\"") {
        if let Some(jar) = args.get(jar_idx + 1) {
            let jar_str = jar.to_string_lossy();
            if let Some(file) = jar_str.split(['/', '\\']).next_back() {
                return file.to_string();
            }
        }
    }

    // 否则找 main class：最后一个像包名的位置参数（含 `.` 且无路径分隔符）
    // 形如 -cp ... com.example.Application
    let mut last_class: Option<String> = None;
    for arg in args {
        let s = arg.to_string_lossy();
        if s.starts_with('-') {
            continue;
        }
        // main class 特征：含 . 分隔的标识符，无路径分隔符，不以 .jar 结尾
        if s.contains('.')
            && !s.contains('/')
            && !s.contains('\\')
            && !s.ends_with(".jar")
            && !s.ends_with(".xml")
            && !s.ends_with(".properties")
        {
            let cls = match s.rsplit_once('.') {
                Some((_pkg, cls)) => cls.to_string(),
                None => s.into_owned(),
            };
            last_class = Some(cls);
        }
    }
    if let Some(cls) = last_class {
        return cls;
    }
    String::new()
}

/// 从 Node 命令行参数提取主脚本名。
fn extract_node_main_script(args: &[&std::ffi::OsString], name: &str) -> String {
    let name_lower = name.to_ascii_lowercase();
    let mut skip_next = false;
    let mut script: Option<String> = None;

    for arg in args {
        let s = arg.to_string_lossy();

        // 上一轮是带值选项（如 -e "code"），跳过当前值
        if skip_next {
            skip_next = false;
            continue;
        }

        // 选项参数：跳过，部分带值选项标记跳过下一个
        if s.starts_with('-') {
            // node 的 -e/--eval <code>、-r/--require <module>、--inspect 等带值
            let lower = s.to_ascii_lowercase();
            if lower == "-e" || lower == "--eval" || lower == "-r" || lower == "--require" {
                skip_next = true;
            }
            continue;
        }

        // 第一个位置参数 = 正在执行的脚本
        if script.is_none() {
            script = Some(s.to_string());
        }
    }

    let Some(script) = script else {
        return String::new();
    };

    // npm/yarn/pnpm run <script>：取 script 名而非 npm.exe
    if name_lower == "npm.exe" || name_lower == "npm" {
        // 形如 npm run dev → 找 "run" 后的 script 名
        // 但通常 npm 会 spawn node 执行，这里 script 可能是 npm-cli 路径
        // 取文件名即可
    }

    // 取文件名（去掉路径），保留扩展名
    let file = script.split(['/', '\\']).next_back().unwrap_or(&script);
    file.to_string()
}

/// 提取命令行语义化摘要（cmdline_summary）——比 main_script 更完整的辨识信息。
///
/// 与 main_script（纯程序名）的区别：cmdline_summary 包含关键参数，
/// 让用户看到「进程在怎么跑」。如 `vite.js --port 5173`、`-jar demo.jar --server.port=8080`。
fn extract_cmdline_summary(cmd: &[std::ffi::OsString], kind: &str) -> String {
    if cmd.len() < 2 {
        return String::new();
    }
    let args: Vec<&std::ffi::OsString> = cmd.iter().skip(1).collect();

    // 收集有辨识度的片段：主脚本 + 关键参数（端口、环境、配置）
    let mut parts: Vec<String> = Vec::new();
    let mut main_added = false;

    let mut skip_next = false;
    for arg in &args {
        let s = arg.to_string_lossy();

        if skip_next {
            skip_next = false;
            // 带值选项的值通常无辨识度，跳过（除非是端口等，但 -p <port> 这种少见）
            continue;
        }

        if s.starts_with('-') {
            let lower = s.to_ascii_lowercase();
            if kind == "java" && (lower == "-e" || lower == "--eval") {
                skip_next = true;
                continue;
            }
            if lower == "-r" || lower == "--require" {
                skip_next = true;
                continue;
            }
            // 保留有辨识度的选项参数
            if is_meaningful_arg(&s, kind) {
                parts.push(s.to_string());
            }
            continue;
        }

        // 位置参数
        if !main_added {
            // 第一个位置参数是主脚本/程序，取文件名
            let file = s.split(['/', '\\']).next_back().unwrap_or(&s);
            parts.push(file.to_string());
            main_added = true;
        } else if is_meaningful_arg(&s, kind) {
            // 后续位置参数也可能有意义（如 npm script 名 dev/build）
            parts.push(s.to_string());
        }
    }

    if parts.is_empty() {
        return String::new();
    }
    // 截断到合理长度
    let summary = parts.join(" ");
    if summary.len() > 120 {
        let truncated: String = summary.chars().take(120).collect();
        format!("{truncated}…")
    } else {
        summary
    }
}

/// 判断一个命令行参数是否有辨识度（值得放进 summary）。
fn is_meaningful_arg(arg: &str, kind: &str) -> bool {
    let lower = arg.to_ascii_lowercase();
    // 端口、环境、配置类参数
    if lower.contains("port")
        || lower.contains("host")
        || lower.contains("mode")
        || lower.contains("env")
        || lower.contains("config")
    {
        return true;
    }
    // Spring Boot 参数
    if lower.contains("server.port") || lower.contains("spring.profiles") {
        return true;
    }
    // node 常见有意义的位置参数（npm script 名）
    if kind == "node" {
        match lower.as_str() {
            "dev" | "serve" | "start" | "build" | "watch" | "test" | "preview" => return true,
            _ => {}
        }
    }
    // 含路径分隔符的通常是文件路径，意义不大（太长），跳过
    if arg.contains('/') || arg.contains('\\') {
        return false;
    }
    // classpath 值等通常无辨识度，跳过
    if lower.starts_with("-cp") || lower.starts_with("-classpath") {
        return false;
    }
    false
}

/// 提取进程关联的项目/工作目录路径（project_path）。
///
/// 这是用户判断「该不该清理」的关键信息。提取策略（按分类）：
/// - IDE：从命令行参数提取打开的工作区/文件夹路径
/// - Java 服务：从 `-jar` 路径提取所在目录
/// - Node dev server：从 node_modules 路径提取项目根目录
/// - Gradle/Maven：从命令行参数提取项目路径
fn extract_project_path(
    cmd: &[std::ffi::OsString],
    exe: &str,
    category: DevProcCategory,
    cwd: Option<&str>,
) -> String {
    // ===== IDE：提取工作区路径 =====
    if category == DevProcCategory::IdeMain {
        // VSCode：--folder-uri file:///path 或 位置参数中的目录路径
        if let Some(path) = extract_vscode_workspace(cmd) {
            return path;
        }
        // JetBrains：命令行最后的位置参数通常是项目目录
        // idea64.exe ... /path/to/project
        if let Some(path) = extract_positional_dir(cmd, exe) {
            return path;
        }
        return String::new();
    }

    // ===== Java 服务（-jar 路径的所在目录）=====
    if let Some(jar_idx) = cmd.iter().position(|s| s == "-jar") {
        if let Some(jar) = cmd.get(jar_idx + 1) {
            let jar_str = jar.to_string_lossy();
            if let Some(dir) = parent_dir(&jar_str) {
                return dir;
            }
        }
    }

    // ===== Node：node_modules 路径的项目根 =====
    if let Some(path) = cmd.iter().find_map(|s| {
        let s = s.to_string_lossy();
        let nm_idx = s.find("node_modules")?;
        let dir = s[..nm_idx]
            .split(['/', '\\'])
            .filter(|p| !p.is_empty())
            .next_back()?;
        Some(dir.to_string())
    }) {
        return path;
    }

    // ===== Gradle/Maven：从 build.gradle / pom.xml 路径提取项目目录 =====
    if category == DevProcCategory::BuildDaemon {
        if let Some(path) = cmd.iter().find_map(|s| {
            let s = s.to_string_lossy();
            for marker in &["build.gradle", "settings.gradle", "pom.xml"] {
                if s.contains(marker) {
                    return parent_dir(&s);
                }
            }
            None
        }) {
            return path;
        }
    }

    // 所有命令行提取都失败时，回退到进程工作目录（cwd）
    // cwd 是 OS 报告的真实工作目录，比命令行推断更权威
    if let Some(c) = cwd {
        if !c.is_empty() {
            return c.to_string();
        }
    }

    String::new()
}

/// 提取 VSCode 命令行中的工作区路径。
/// 支持两种参数形式：`--folder-uri=value` 和 `--folder-uri value`（独立 token）。
fn extract_vscode_workspace(cmd: &[std::ffi::OsString]) -> Option<String> {
    let mut iter = cmd.iter().peekable();
    while let Some(s) = iter.next() {
        let s = s.to_string_lossy();
        let lower = s.to_ascii_lowercase();
        if lower.starts_with("--folder-uri") || lower.starts_with("--file-uri") {
            // 形式 1：--folder-uri=value
            if let Some(val) = s.splitn(2, '=').nth(1) {
                return Some(decode_file_uri(val));
            }
            // 形式 2：--folder-uri <value>（值为下一个 token）
            if let Some(next) = iter.peek() {
                let next = next.to_string_lossy();
                if !next.starts_with('-') {
                    return Some(decode_file_uri(&next));
                }
            }
        }
    }
    // 位置参数中的目录路径（非 .exe 开头的路径参数）
    extract_positional_dir(cmd, "")
}

/// 从位置参数中提取看起来像项目目录的路径。
/// 跳过以 `-` 开头的选项参数和 exe 本身。
fn extract_positional_dir(cmd: &[std::ffi::OsString], exe: &str) -> Option<String> {
    let exe_lower = exe.to_ascii_lowercase();
    for s in cmd.iter().skip(1) {
        // 跳过第一个（通常是 exe 路径）
        let s = s.to_string_lossy();
        if s.starts_with('-') {
            continue;
        }
        let lower = s.to_ascii_lowercase();
        // 跳过 exe 本身、.code-workspace 之外的纯文件
        if lower == exe_lower || lower.ends_with(".exe") {
            continue;
        }
        // 含路径分隔符 → 可能是项目目录
        if s.contains('/') || s.contains('\\') {
            return Some(s.to_string());
        }
        // 无分隔符但像目录名（IDEA 有时传纯目录名）
        if !s.contains('.') && s.len() > 1 {
            return Some(s.to_string());
        }
    }
    None
}

/// 从 file:/// URI 解码出本地路径。
fn decode_file_uri(uri: &str) -> String {
    let s = uri.strip_prefix("file://").unwrap_or(uri);
    // Windows 路径 file:///C:/... → 去掉前导 /
    let s = s.strip_prefix('/').unwrap_or(s);
    // 还原百分号编码（%20 → 空格等，简化处理常见情况）
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex1 = chars.next();
            let hex2 = chars.next();
            if let (Some(h1), Some(h2)) = (hex1, hex2) {
                if let Ok(byte) = u8::from_str_radix(&format!("{h1}{h2}"), 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push(c);
        } else {
            result.push(c);
        }
    }
    result
}

/// 取路径的父目录（去掉最后一段）。如 /a/b/c.jar → /a/b
fn parent_dir(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches(['/', '\\']);
    match trimmed.rsplit_once(['/', '\\']) {
        Some((dir, _file)) if !dir.is_empty() => Some(dir.to_string()),
        _ => None,
    }
}

/// 从命令行提取项目/模块提示，方便用户辨认进程归属。
///
/// 优先级：
/// - Maven artifactId（`-Dproject.artifactId=` 或 pom 的 artifact）
/// - Gradle 项目（路径中的项目目录）
/// - package.json 的 name（命令行中的项目路径）
/// - 兜底：exe 文件名
fn extract_hint(cmd: &[std::ffi::OsString], exe: &str, name: &str) -> String {
    let cmdline_lower = join_cmdline(cmd).to_ascii_lowercase();

    // Maven：-Dclassworlds ... 或直接找 artifact
    if let Some(idx) = cmdline_lower.find("artifactid=") {
        let rest = &cmdline_lower[idx + "artifactid=".len()..];
        let artifact: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
        if !artifact.is_empty() {
            return artifact;
        }
    }

    // Gradle daemon：命令行可能带项目路径（含 build.gradle / settings.gradle）
    if cmdline_lower.contains("gradle") {
        if let Some(seg) = cmd.iter().find_map(|s| {
            let s = s.to_string_lossy();
            // 形如 /path/to/myproject/build.gradle —— 项目目录是倒数第二段
            if s.contains("build.gradle") || s.contains("settings.gradle") {
                let parts: Vec<&str> = s.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
                // 倒数第二段是项目目录（最后一段是 build.gradle / settings.gradle）
                if parts.len() >= 2 {
                    return Some(parts[parts.len() - 2].to_string());
                }
            }
            None
        }) {
            return seg;
        }
        return "Gradle".to_string();
    }

    // Spring Boot：从 -jar 路径提取 jar 名
    if let Some(jar_idx) = cmd.iter().position(|s| s == "-jar") {
        if let Some(jar) = cmd.get(jar_idx + 1) {
            let jar_str = jar.to_string_lossy();
            if let Some(file) = jar_str.split(['/', '\\']).next_back() {
                return file.to_string();
            }
        }
    }

    // Node dev server：从脚本路径提取项目目录
    // 形如 node /path/to/project/node_modules/vite/bin/vite.js
    if let Some(path_seg) = cmd.iter().find_map(|s| {
        let s = s.to_string_lossy();
        if s.contains("node_modules") || s.ends_with(".js") || s.ends_with(".mjs") {
            // 从路径中提取项目根（node_modules 的前一段）
            if let Some(nm_idx) = s.find("node_modules") {
                let before = &s[..nm_idx];
                return before
                    .split(['/', '\\'])
                    .filter(|p| !p.is_empty())
                    .next_back()
                    .map(String::from);
            }
            s.split(['/', '\\']).filter(|p| !p.is_empty()).next_back().map(String::from)
        } else {
            None
        }
    }) {
        return path_seg;
    }

    // 兜底：exe 文件名
    if !exe.is_empty() {
        if let Some(file) = exe.split(['/', '\\']).next_back() {
            return file.to_string();
        }
    }
    name.to_string()
}

/// Unix 时间戳（秒）转 ISO 字符串（本地时区）。
fn unix_to_iso_string(secs: u64) -> Option<String> {
    use chrono::{Local, TimeZone};
    let dt = Local.timestamp_opt(secs as i64, 0).single()?;
    Some(dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_java_node_names() {
        assert!(is_java_process("java.exe"));
        assert!(is_java_process("javaw.exe"));
        assert!(is_java_process("java"));
        assert!(!is_java_process("node.exe"));
    }

    #[test]
    fn build_daemon_detection() {
        assert!(is_build_daemon(
            "java.exe",
            "org.gradle.launcher.daemon.bootstrap"
        ));
        assert!(is_build_daemon(
            "java.exe",
            "org.codehaus.plexus.classworlds.launcher.launcher"
        ));
        assert!(!is_build_daemon("java.exe", "vite"));
    }

    #[test]
    fn dev_server_detection() {
        assert!(is_dev_server("node vite --port 5173"));
        assert!(is_dev_server("webpack serve --mode development"));
        assert!(is_dev_server("spring-boot:run"));
        assert!(!is_dev_server("npm install"));
    }

    #[test]
    fn watcher_detection() {
        assert!(is_watcher("tsc --watch"));
        assert!(is_watcher("nodemon server.js"));
        assert!(!is_watcher("node server.js"));
    }

    #[test]
    fn truncate_long_cmdline() {
        let long = "x".repeat(500);
        let t = truncate_cmdline(&long);
        assert!(t.ends_with('…'));
        assert!(t.chars().count() == CMDLINE_DISPLAY_MAX + 1); // +1 for ellipsis
    }

    #[test]
    fn extract_hint_gradle() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java".into(),
            "-Dorg.gradle.launcher.daemon".into(),
            "/home/user/myproject/build.gradle".into(),
        ];
        let hint = extract_hint(&cmd, "/usr/bin/java", "java");
        // 命令行含 gradle → 应返回项目目录
        assert_eq!(hint, "myproject");
    }

    #[test]
    fn extract_hint_jar() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java".into(),
            "-jar".into(),
            "/opt/app/demo-service-1.0.jar".into(),
        ];
        let hint = extract_hint(&cmd, "/usr/bin/java", "java");
        assert_eq!(hint, "demo-service-1.0.jar");
    }

    #[test]
    fn extract_hint_fallback_exe_name() {
        let cmd: Vec<std::ffi::OsString> = vec!["node".into()];
        let hint = extract_hint(&cmd, "C:\\Program Files\\nodejs\\node.exe", "node.exe");
        assert_eq!(hint, "node.exe");
    }

    #[test]
    fn recommend_orphan_always() {
        assert!(is_recommended(DevProcCategory::Orphan, 1024, 1234, None));
    }

    #[test]
    fn recommend_ide_never() {
        assert!(!is_recommended(DevProcCategory::IdeMain, 999_999_999, 1234, None));
    }

    #[test]
    fn recommend_build_daemon_threshold() {
        assert!(!is_recommended(
            DevProcCategory::BuildDaemon,
            100 * 1024 * 1024,
            1,
            None
        ));
        assert!(is_recommended(
            DevProcCategory::BuildDaemon,
            600 * 1024 * 1024,
            1,
            None
        ));
    }

    #[test]
    fn recommend_dev_server_threshold() {
        assert!(!is_recommended(
            DevProcCategory::DevServer,
            500 * 1024 * 1024,
            1,
            None
        ));
        assert!(is_recommended(
            DevProcCategory::DevServer,
            2 * 1024 * 1024 * 1024,
            1,
            None
        ));
    }

    #[test]
    fn scan_empty_system_returns_empty() {
        let system = System::new();
        let result = scan_dev_processes(&system, None);
        assert!(result.is_empty());
    }

    /// 验证 classify 对不存在的 PID 优雅降级：空 system 下分类为 Orphan（父链无 IDE）。
    #[test]
    fn classify_handles_missing_process_gracefully() {
        let system = System::new();
        let ide_pids = HashSet::new();
        let cat = classify(
            999999,
            "node.exe",
            "C:\\node.exe",
            &[],
            None,
            &ide_pids,
            &system,
        );
        // pid 不存在但函数仍应返回某分类（Orphan，因父链无 IDE）
        assert_eq!(cat, DevProcCategory::Orphan);
    }

    /// 校验 aggregate_tree 对不存在 PID 返回零值。
    #[test]
    fn aggregate_unknown_pid_zero() {
        let system = System::new();
        let (cpu, mem) = aggregate_tree(&[0xFFFF_FFFF], &system);
        assert_eq!((cpu, mem), (0.0, 0));
    }

    // ===== display_title 测试 =====

    #[test]
    fn display_title_idea() {
        let cmd: Vec<std::ffi::OsString> = vec!["idea64.exe".into()];
        let title = extract_display_title(
            "idea64.exe",
            "C:\\Program Files\\JetBrains\\IntelliJ IDEA\\bin\\idea64.exe",
            &cmd,
            DevProcCategory::IdeMain,
            "",
        );
        assert_eq!(title, "IntelliJ IDEA");
    }

    #[test]
    fn display_title_vscode_main() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "Code.exe".into(),
            "--folder-uri".into(),
            "file:///C:/Users/proj".into(),
        ];
        let title = extract_display_title(
            "Code.exe",
            "C:\\Users\\user\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe",
            &cmd,
            DevProcCategory::IdeMain,
            "",
        );
        assert_eq!(title, "VSCode 主进程");
    }

    #[test]
    fn display_title_vscode_extension_host() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "Code.exe".into(),
            "--type=extensionHost".into(),
        ];
        let title = extract_display_title(
            "Code.exe",
            "C:\\AppData\\Microsoft VS Code\\Code.exe",
            &cmd,
            DevProcCategory::IdeMain,
            "",
        );
        assert_eq!(title, "VSCode 扩展宿主");
    }

    #[test]
    fn display_title_webstorm() {
        let title = extract_display_title(
            "webstorm64.exe",
            "C:\\Program Files\\JetBrains\\WebStorm\\bin\\webstorm64.exe",
            &[],
            DevProcCategory::IdeMain,
            "",
        );
        assert_eq!(title, "WebStorm");
    }

    #[test]
    fn display_title_gradle_daemon() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java.exe".into(),
            "-Dorg.gradle.launcher.daemon".into(),
        ];
        let title = extract_display_title("java.exe", "java", &cmd, DevProcCategory::BuildDaemon, "");
        assert_eq!(title, "Gradle Daemon");
    }

    #[test]
    fn display_title_vite_dev() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "node.exe".into(),
            "vite".into(),
            "--port".into(),
            "5173".into(),
        ];
        let title = extract_display_title("node.exe", "node", &cmd, DevProcCategory::DevServer, "");
        assert_eq!(title, "vite dev server");
    }

    // ===== project_path 测试 =====

    #[test]
    fn project_path_from_jar() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java.exe".into(),
            "-jar".into(),
            "/opt/app/demo-service.jar".into(),
        ];
        let path = extract_project_path(&cmd, "java", DevProcCategory::Other, None);
        assert_eq!(path, "/opt/app");
    }

    #[test]
    fn project_path_from_node_modules() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "node.exe".into(),
            "D:\\projects\\myapp\\node_modules\\vite\\bin\\vite.js".into(),
        ];
        let path = extract_project_path(&cmd, "node", DevProcCategory::DevServer, None);
        assert_eq!(path, "myapp");
    }

    #[test]
    fn project_path_vscode_workspace() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "Code.exe".into(),
            "--folder-uri".into(),
            "file:///C:/Users/dev/myproject".into(),
        ];
        let path = extract_project_path(&cmd, "C:\\VSCode\\Code.exe", DevProcCategory::IdeMain, None);
        assert_eq!(path, "C:/Users/dev/myproject");
    }

    #[test]
    fn project_path_gradle_build_file() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java.exe".into(),
            "/home/user/myproject/build.gradle".into(),
        ];
        let path = extract_project_path(&cmd, "java", DevProcCategory::BuildDaemon, None);
        assert_eq!(path, "/home/user/myproject");
    }

    #[test]
    fn project_path_fallback_to_cwd() {
        let cmd: Vec<std::ffi::OsString> = vec!["node.exe".into()];
        let path = extract_project_path(&cmd, "node", DevProcCategory::Other, Some("D:\\project"));
        assert_eq!(path, "D:\\project");
    }

    #[test]
    fn project_path_empty_for_minimal_cmd() {
        let cmd: Vec<std::ffi::OsString> = vec!["node.exe".into()];
        let path = extract_project_path(&cmd, "node", DevProcCategory::Other, None);
        assert!(path.is_empty());
    }

    #[test]
    fn decode_file_uri_spaces() {
        assert_eq!(
            decode_file_uri("file:///C:/Users/My%20Documents/proj"),
            "C:/Users/My Documents/proj"
        );
    }

    #[test]
    fn parent_dir_basic() {
        assert_eq!(parent_dir("/a/b/c.jar"), Some("/a/b".to_string()));
        assert_eq!(parent_dir("C:\\app\\demo.jar"), Some("C:\\app".to_string()));
        assert_eq!(parent_dir("file.jar"), None);
    }

    // ===== main_script 测试 =====

    #[test]
    fn main_script_node_js_file() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "node.exe".into(),
            "D:\\project\\server.js".into(),
            "--port".into(),
            "3000".into(),
        ];
        let script = extract_main_script(&cmd, "node", "node.exe");
        assert_eq!(script, "server.js");
    }

    #[test]
    fn main_script_node_vite() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "node.exe".into(),
            "D:\\project\\node_modules\\vite\\bin\\vite.js".into(),
        ];
        let script = extract_main_script(&cmd, "node", "node.exe");
        assert_eq!(script, "vite.js");
    }

    #[test]
    fn main_script_java_jar() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java.exe".into(),
            "-jar".into(),
            "/opt/app/demo-service.jar".into(),
        ];
        let script = extract_main_script(&cmd, "java", "java.exe");
        assert_eq!(script, "demo-service.jar");
    }

    #[test]
    fn main_script_java_main_class() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java.exe".into(),
            "-cp".into(),
            "lib/*".into(),
            "com.example.Application".into(),
        ];
        let script = extract_main_script(&cmd, "java", "java.exe");
        assert_eq!(script, "Application");
    }

    #[test]
    fn main_script_empty_for_bare_process() {
        let cmd: Vec<std::ffi::OsString> = vec!["node.exe".into()];
        let script = extract_main_script(&cmd, "node", "node.exe");
        assert!(script.is_empty());
    }

    // ===== cmdline_summary 测试 =====

    #[test]
    fn summary_node_with_port() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "node.exe".into(),
            "vite.js".into(),
            "--port".into(),
            "5173".into(),
        ];
        let summary = extract_cmdline_summary(&cmd, "node");
        assert!(summary.contains("vite.js"));
        assert!(summary.contains("--port"));
    }

    #[test]
    fn summary_java_jar_with_config() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java.exe".into(),
            "-jar".into(),
            "app.jar".into(),
            "--spring.profiles.active=dev".into(),
        ];
        let summary = extract_cmdline_summary(&cmd, "java");
        assert!(summary.contains("app.jar"));
        assert!(summary.contains("spring.profiles"));
    }

    #[test]
    fn summary_truncates_long() {
        let long_arg = "x".repeat(150);
        let cmd: Vec<std::ffi::OsString> = vec!["node.exe".into(), long_arg.into()];
        let summary = extract_cmdline_summary(&cmd, "node");
        assert!(summary.ends_with('…'));
    }

    // ===== display_title Other 分支（用 main_script）测试 =====

    #[test]
    fn display_title_other_node_with_script() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "node.exe".into(),
            "server.js".into(),
        ];
        let title = extract_display_title(
            "node.exe",
            "C:\\node.exe",
            &cmd,
            DevProcCategory::Other,
            "server.js",
        );
        assert_eq!(title, "server.js (node)");
    }

    #[test]
    fn display_title_other_java_with_class() {
        let cmd: Vec<std::ffi::OsString> = vec![
            "java.exe".into(),
            "com.example.App".into(),
        ];
        let title = extract_display_title(
            "java.exe",
            "java",
            &cmd,
            DevProcCategory::Orphan,
            "App",
        );
        assert_eq!(title, "App (java)");
    }

    // ===== is_meaningful_arg 测试 =====

    #[test]
    fn meaningful_arg_detection() {
        assert!(is_meaningful_arg("--port=5173", "node"));
        assert!(is_meaningful_arg("--server.port=8080", "java"));
        assert!(is_meaningful_arg("dev", "node"));
        assert!(!is_meaningful_arg("some-random-arg", "node"));
        assert!(!is_meaningful_arg("D:\\path\\to\\file", "node"));
    }
}
