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
        let cmdline_hint = extract_hint(cmdline_vec, &exe, &name);

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

        let category = classify(
            pid_u32,
            &name,
            &exe,
            cmdline_vec,
            ppid,
            &ide_pids,
            system,
        );

        let recommended = is_recommended(category, tree_mem, pid_u32, self_pid);

        results.push(DevProcInfo {
            pid: pid_u32,
            ppid,
            kind: kind.into(),
            name,
            exe,
            cmdline: cmdline_display,
            cmdline_hint,
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
}
