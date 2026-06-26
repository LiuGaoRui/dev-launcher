//! 项目自动检测服务
//!
//! 给定根目录，递归扫描其中的 Java(Maven) / Node 项目，并为每个项目推断
//! 启动命令、构建命令、预期端口。
//!
//! 设计：
//! - 纯业务逻辑，不依赖 Tauri / DB，可直接单测
//! - 有界 DFS（最大深度 4 层）+ 目录跳过清单，避免误入 node_modules / target 等
//! - 目录命中标志文件即判定为项目，不再下钻该目录子项（避免重复检测）
//! - monorepo 根（package.json workspaces 或 pom 聚合器）只做容器，继续下钻找子模块
//! - pom / package.json / application 配置均用字符串/JSON 方式轻量解析，不引入重型 XML 库

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;

use crate::error::{AppError, AppResult};
use crate::models::{DetectedProject, LaunchScheme, ProjectType};

/// 扫描跳过的目录名（不分大小写匹配）。
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    ".idea",
    ".gradle",
    "__pycache__",
    ".next",
    ".nuxt",
    ".vscode",
    ".cache",
    "venv",
    ".venv",
];

/// DFS 最大深度（根目录为第 0 层）。
const MAX_DEPTH: u32 = 4;

/// `detect_maven` 的探测结果。
enum MavenProbe {
    /// 聚合器（`<packaging>pom</packaging>`），作为 monorepo 容器继续下钻
    Aggregator,
    /// 可执行项目（SpringBoot 或带可执行 jar 标志的普通 Maven），加入结果
    Project(DetectedProject),
    /// 纯类库（jar，不可独立启动），不入结果但停止下钻
    Library,
}

pub struct DetectService;

impl DetectService {
    /// 扫描根目录下所有可识别的项目。
    ///
    /// 用户取消或路径不存在返回 `AppError::Io`；无项目返回空 Vec。
    pub async fn scan(root: &str) -> AppResult<Vec<DetectedProject>> {
        let root_path = PathBuf::from(root);
        if !root_path.exists() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("扫描目录不存在: {root}"),
            )));
        }
        if !root_path.is_dir() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("扫描路径不是目录: {root}"),
            )));
        }

        // 同步遍历：桌面端用户手动触发、有界深度 + skip-list，通常亚秒级。
        // 不引入 spawn_blocking 以保持实现简单。
        let mut results: Vec<DetectedProject> = Vec::new();
        Self::walk(&root_path, &root_path, 0, &mut results)?;
        // 按相对路径排序，保证多次扫描结果稳定
        results.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(results)
    }

    /// 递归遍历单个目录。
    ///
    /// `root` 为扫描根（用于计算 rel_path）；`dir` 为当前目录；`depth` 为当前深度。
    fn walk(root: &Path, dir: &Path, depth: u32, out: &mut Vec<DetectedProject>) -> AppResult<()> {
        if depth > MAX_DEPTH {
            return Ok(());
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                // 单个目录无权限等不致命，跳过
                tracing::debug!(path = ?dir, error = %e, "读取目录失败，跳过");
                return Ok(());
            }
        };

        // 先收集本目录下的直接子项，判断是否为项目根 / monorepo 容器
        let mut child_dirs: Vec<PathBuf> = Vec::new();
        let mut has_pom = false;
        let mut has_pkg = false;
        let mut pkg_path: Option<PathBuf> = None;

        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_dir() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if SKIP_DIRS.iter().any(|s| *s == name) {
                    continue;
                }
                // 跳过隐藏目录（以 . 开头）
                if name.starts_with('.') {
                    continue;
                }
                child_dirs.push(path);
            } else if file_type.is_file() {
                let name = entry.file_name();
                if name == "pom.xml" {
                    has_pom = true;
                } else if name == "package.json" {
                    has_pkg = true;
                    pkg_path = Some(path);
                }
            }
        }

        // 判定本目录是否为「项目根」
        if has_pom {
            // pom.xml：Maven 项目。聚合器视为 monorepo 容器继续下钻
            match Self::detect_maven(root, dir)? {
                Some(MavenProbe::Project(d)) => {
                    out.push(d);
                    return Ok(());
                }
                Some(MavenProbe::Library) => { /* 纯类库 jar，不入结果，停止下钻 */ }
                Some(MavenProbe::Aggregator) => { /* monorepo 容器：继续下钻子目录 */ }
                None => { /* 非 Maven 项目根，继续下钻 */ }
            }
        } else if has_pkg {
            if let Some(pkg_path) = pkg_path {
                // package.json 只读一次，复用给 workspace 判定 + 项目检测
                if let Some(pkg) = read_pkg_json(&pkg_path) {
                    if Self::is_node_workspace(&pkg) {
                        // monorepo 根：继续下钻找子包
                    } else if let Some(detected) = Self::detect_node(root, dir, &pkg)? {
                        out.push(detected);
                        return Ok(());
                    }
                }
            }
        }

        // 继续下钻子目录
        for child in child_dirs {
            Self::walk(root, &child, depth + 1, out)?;
        }
        Ok(())
    }

    // ===== Node 检测 =====

    /// 判断 package.json 是否为 monorepo workspace 根。
    fn is_node_workspace(pkg: &serde_json::Value) -> bool {
        pkg.get("workspaces").is_some()
    }

    /// 检测 Node 项目，返回 None 表示非有效项目（如无可运行脚本）。
    fn detect_node(root: &Path, dir: &Path, pkg: &serde_json::Value) -> AppResult<Option<DetectedProject>> {
        // 项目名：package.json name → 目录名
        let name = pkg
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                dir.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "node-project".to_string())
            });

        // 包管理器前缀
        let pm = Self::detect_pkg_manager(dir);

        // scripts
        let scripts = pkg.get("scripts");
        let has_script = |key: &str| -> bool {
            scripts
                .and_then(|s| s.get(key))
                .and_then(|v| v.as_str())
                .is_some()
        };

        // 推断端口：优先读配置文件实际端口，再按依赖兜底默认值
        let ports = Self::detect_node_ports(dir, pkg);

        // 构建启动命令
        let dev_cmd = if has_script("dev") {
            Some(format!("{pm} run dev"))
        } else if has_script("start") {
            Some(format!("{pm} start"))
        } else {
            None
        };

        // 无可运行脚本，不算可启动项目
        let start_cmd = match dev_cmd {
            Some(c) => c,
            None => return Ok(None),
        };

        let build_cmd = if has_script("build") {
            Some(format!("{pm} run build"))
        } else {
            None
        };

        let schemes = vec![LaunchScheme {
            label: "开发模式".to_string(),
            recommended: true,
            start_cmd,
            build_cmd,
            description: "直接运行 dev/start 脚本，支持热更新".to_string(),
        }];

        Ok(Some(DetectedProject {
            rel_path: relpath(root, dir),
            path: dir.to_string_lossy().to_string(),
            workdir: root.to_string_lossy().to_string(),
            name,
            r#type: ProjectType::Node,
            expected_ports: ports,
            schemes,
        }))
    }

    /// 根据 lock 文件推断包管理器。
    fn detect_pkg_manager(dir: &Path) -> &'static str {
        if dir.join("pnpm-lock.yaml").exists() {
            "pnpm"
        } else if dir.join("yarn.lock").exists() {
            "yarn"
        } else {
            "npm"
        }
    }

    /// 推断 Node 项目端口：优先从配置文件读取实际端口，找不到再按依赖兜底默认值。
    ///
    /// 读取顺序：① vue-cli 的 `vue.config.js`（含 `|| <port>` 或 `port: <port>`）
    ///          ② vite 的 `vite.config.[js|ts]`（`server.port`）
    /// 都没有则按依赖给框架默认端口（vite→5173、@vue/cli-service→8080 等）。
    fn detect_node_ports(dir: &Path, pkg: &serde_json::Value) -> Vec<String> {
        let in_deps = |section: &str, name: &str| {
            pkg.get(section).and_then(|v| v.get(name)).is_some()
        };
        let has = |n: &str| in_deps("dependencies", n) || in_deps("devDependencies", n);

        // vue-cli：读 vue.config.js 实际端口
        if has("@vue/cli-service") {
            if let Some(p) = read_port_from_vue_config(dir) {
                return vec![p];
            }
            return vec!["8080".to_string()];
        }
        // vite：读 vite.config 实际端口
        if has("vite") {
            if let Some(p) = read_port_from_vite_config(dir) {
                return vec![p];
            }
            return vec!["5173".to_string()];
        }
        // 其他框架只有默认值，无配置文件可读
        if has("@angular/cli") {
            vec!["4200".to_string()]
        } else if has("next") || has("react-scripts") || has("nuxt") {
            vec!["3000".to_string()]
        } else {
            Vec::new()
        }
    }

    // ===== Maven 检测 =====

    /// 检测 Maven 项目。
    ///
    /// 返回：
    /// - `Ok(None)` — 不是 Maven 项目根（无 pom.xml 或读取失败）
    /// - `Ok(Some(Aggregator))` — 聚合器（`<packaging>pom</packaging>`），继续下钻
    /// - `Ok(Some(Library))` — 非启动模块（无 main 或无端口），不入结果
    /// - `Ok(Some(Project(d)))` — 启动模块（有 main 入口 + 有端口）
    fn detect_maven(root: &Path, dir: &Path) -> AppResult<Option<MavenProbe>> {
        let pom_path = dir.join("pom.xml");
        let content = match std::fs::read_to_string(&pom_path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        // 一次预处理：去注释 + 删 <parent> 块，供后续所有 pom 解析复用
        let stripped = strip_pom(&content);

        // 聚合器：packaging=pom 视为 monorepo 容器，不下钻入结果
        if is_pom_aggregator(&stripped) {
            return Ok(Some(MavenProbe::Aggregator));
        }

        // ===== 启动模块双硬条件：必须有 main 入口 + 必须有端口 =====
        // 没有 main 方法 → 类库/common 模块，不可独立启动
        if !has_main_class(dir) {
            return Ok(Some(MavenProbe::Library));
        }
        // 没有可识别端口 → 无法健康检查，不算启动模块
        let expected_ports = Self::read_java_port(dir);
        if expected_ports.is_empty() {
            return Ok(Some(MavenProbe::Library));
        }

        // 通过双硬条件后，按是否含 spring-boot 细分类型（用于启动方案生成）
        let is_spring_boot = content.contains("spring-boot");
        let ptype = if is_spring_boot {
            ProjectType::Springboot
        } else {
            ProjectType::JavaJar
        };

        // 项目名：跳过 <parent> 块后取首个 <artifactId>
        let name = extract_pom_artifact_id(&stripped)
            .unwrap_or_else(|| {
                dir.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "maven-project".to_string())
            });

        // jar 名：优先扫 target/*.jar 实际产物；其次 pom 坐标；最后占位
        let jar_name = Self::resolve_jar_name(dir, &stripped);
        // 模块相对根目录路径（用于 java -jar 时从根目录指向模块 target）
        let mod_rel = relpath(root, dir);
        let root_str = root.to_string_lossy().to_string();

        let schemes = Self::build_maven_schemes(is_spring_boot, &jar_name, &root_str, &mod_rel);

        Ok(Some(MavenProbe::Project(DetectedProject {
            rel_path: mod_rel,
            path: dir.to_string_lossy().to_string(),
            workdir: root_str,
            name,
            r#type: ptype,
            expected_ports,
            schemes,
        })))
    }

    /// 解析 jar 名：扫 target 目录实际产物 > pom 坐标 > None。
    fn resolve_jar_name(dir: &Path, pom_content: &str) -> Option<String> {
        // 1. 扫 target/*.jar（排除 sources/javadoc）
        if let Some(jar) = scan_target_jar(dir) {
            return Some(jar);
        }
        // 2. pom 坐标 artifactId-version.jar
        if let Some(name) = build_coord_jar_name(pom_content) {
            return Some(name);
        }
        None
    }

    /// 读 Java 应用端口：扫描所有 application 配置文件（含多 profile），
    /// 返回发现的全部端口（去重）。
    ///
    /// 覆盖：`application.properties` / `application.yml` / `application.yaml`
    /// 及 `application-{profile}.properties/.yml/.yaml`。
    fn read_java_port(dir: &Path) -> Vec<String> {
        let res = dir.join("src/main/resources");
        let entries = match std::fs::read_dir(&res) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        // 先默认配置，再 profile 配置；按文件名排序保证多次扫描结果稳定
        let mut files: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("application") {
                files.push(entry.path());
            }
        }
        files.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .cmp(b.file_name().unwrap_or_default())
        });

        let mut ports: Vec<String> = Vec::new();
        for path in files {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let is_properties = path
                .extension()
                .and_then(|e| e.to_str())
                .map_or(false, |e| e.eq_ignore_ascii_case("properties"));
            let port = if is_properties {
                parse_properties_port(&content)
            } else {
                parse_yml_port(&content)
            };
            if let Some(p) = port {
                if !p.is_empty() && !ports.contains(&p) {
                    ports.push(p);
                }
            }
        }
        ports
    }

    /// 生成 Maven 项目的启动方案。
    ///
    /// - `is_spring_boot`：是否 SpringBoot（影响方案数量与命令）
    /// - `jar_name`：推断的 jar 文件名
    /// - `root`：扫描根目录（= workdir，License 等运行时资源在此）
    /// - `mod_rel`：模块相对根目录的路径（如 "backend/hr"）
    ///
    /// 运行目录策略（spawn current_dir 按类型区分，见 spawn.rs）：
    /// - Java 类：current_dir = workdir（root），mvn 用 `-f <mod_rel>/pom.xml` 定位启动模块，
    ///   fork=false 时 user.dir=root 自动找到 License；fork=true 时 workingDirectory 参数生效
    /// - 打包运行：current_dir = workdir（root），jar 在模块 target/ 下，用 mod_rel 指向
    fn build_maven_schemes(
        is_spring_boot: bool,
        jar_name: &Option<String>,
        root: &str,
        mod_rel: &str,
    ) -> Vec<LaunchScheme> {
        let package_build = Some("mvn clean package -DskipTests".to_string());

        // 打包运行时 current_dir=root，jar 在模块 target/ 下，需用相对路径从 root 指向。
        // mod_rel 用 '/' 分隔（relpath 保证），Windows cmd 也接受 '/' 指向子目录。
        let jar_rel_prefix = if mod_rel.is_empty() {
            "target/".to_string()
        } else {
            format!("{mod_rel}/target/")
        };
        let jar_start = match jar_name {
            Some(j) => format!("java -jar {jar_rel_prefix}{j}"),
            None => format!("java -jar {jar_rel_prefix}app.jar"),
        };

        // mvn 用 -f 定位启动模块 pom：spawn cwd=workdir(root)，模块在子目录下。
        // mod_rel 非空（relpath 在 dir==root 时退化为目录名），故总有 <mod_rel>/pom.xml。
        let f_param = format!("-f {mod_rel}/pom.xml ");

        if is_spring_boot {
            let wd_param = format!("-Dspring-boot.run.workingDirectory=\"{root}\"");
            let desc = match jar_name {
                Some(_) => "先构建 jar 再运行，模拟生产形态",
                None => "先构建 jar 再运行；jar 名未能自动识别，请按实际产物修正",
            };
            vec![
                LaunchScheme {
                    label: "开发模式".to_string(),
                    recommended: true,
                    start_cmd: format!("mvn {f_param}spring-boot:run {wd_param}"),
                    build_cmd: None,
                    description: "Maven fork 子进程运行，workingDirectory 设为项目根目录；\
                        改代码重跑即可，日常开发最快。依赖多/路径长时若报 error=206 请用内嵌运行"
                        .to_string(),
                },
                LaunchScheme {
                    label: "开发模式（内嵌运行）".to_string(),
                    recommended: false,
                    // fork=false 时 workingDirectory 参数被 Maven 忽略（仅 fork 模式生效），
                    // 故不写它；user.dir 由 spawn cwd（=workdir root）决定，License 在 root 能找到。
                    start_cmd: format!("mvn {f_param}spring-boot:run -Dspring-boot.run.fork=false"),
                    build_cmd: None,
                    description: "不 fork 子进程，在 Maven 同进程内运行，规避 Windows \
                        classpath 超长（CreateProcess error=206）问题；运行时工作目录=扫描根，\
                        License 等资源须放在扫描根目录"
                        .to_string(),
                },
                LaunchScheme {
                    label: "打包运行模式".to_string(),
                    recommended: false,
                    start_cmd: jar_start,
                    build_cmd: package_build,
                    description: desc.to_string(),
                },
            ]
        } else {
            // 普通 JavaJar：仅打包运行
            let desc = match jar_name {
                Some(_) => "构建 jar 后运行",
                None => "构建 jar 后运行；jar 名未能自动识别，请按实际产物修正",
            };
            vec![LaunchScheme {
                label: "打包运行模式".to_string(),
                recommended: true,
                start_cmd: jar_start,
                build_cmd: package_build,
                description: desc.to_string(),
            }]
        }
    }
}

// ===== 辅助函数 =====

/// 声明一个懒初始化的 Regex 静态变量 + 访问器函数。
/// 用法：`lazy_regex!(my_re, r"pattern");` → 生成 `fn my_re() -> &'static Regex`。
macro_rules! lazy_regex {
    ($name:ident, $pat:literal) => {
        fn $name() -> &'static Regex {
            static RE: OnceLock<Regex> = OnceLock::new();
            RE.get_or_init(|| Regex::new($pat).expect(concat!(stringify!($name), " 正则编译失败")))
        }
    };
}

lazy_regex!(fallback_port_re, r"\|\|\s*(\d{2,5})\b");
lazy_regex!(port_key_value_re, r"(?m)\bport\s*:\s*(\d{2,5})\b");
lazy_regex!(main_re, r"public\s+static\s+void\s+main\s*\(\s*String\s*\[\s*\]\s*\w+\s*\)");
lazy_regex!(java_comment_re, r"//[^\n]*|/\*[\s\S]*?\*/");

/// 读取并解析 package.json；读失败或非法 JSON 返回 None。
fn read_pkg_json(path: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 从 vue.config.js 提取 devServer 端口。
///
/// 覆盖常见写法：
/// - `const port = process.env.port || 8188`（取 `||` 后的数字）
/// - `devServer: { port: 8188 }` 或 `port: 8080,`
/// - `const PORT = process.env.port || process.env.npm_config_port || 9100`
fn read_port_from_vue_config(dir: &Path) -> Option<String> {
    let path = dir.join("vue.config.js");
    let content = std::fs::read_to_string(&path).ok()?;
    extract_port_from_config(&content)
}

/// 从 JS/TS 配置文件文本中提取端口。优先 `|| <port>` 兜底写法，次选 `port: <数字>` 键值。
fn extract_port_from_config(content: &str) -> Option<String> {
    extract_fallback_port_after_or(content).or_else(|| extract_port_key_value(content))
}

/// 从 vite.config.[js|ts|mjs] 提取 server.port。
fn read_port_from_vite_config(dir: &Path) -> Option<String> {
    for fname in &["vite.config.js", "vite.config.ts", "vite.config.mjs"] {
        if let Ok(content) = std::fs::read_to_string(dir.join(fname)) {
            if let Some(port) = extract_port_from_config(&content) {
                return Some(port);
            }
        }
    }
    None
}

/// 提取 `|| <port>` 兜底写法中的端口（取最后一个 `||` 后的数字）。
///
/// 例：`process.env.port || process.env.npm_config_port || 8188` → 8188
fn extract_fallback_port_after_or(content: &str) -> Option<String> {
    // 取最后一个匹配（多级 || 链的最终兜底值）
    fallback_port_re()
        .captures_iter(content)
        .last()
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

/// 提取 `port: <数字>` 键值写法中的端口。
///
/// 例：`port: 8188` / `port: 8080,` / `port: 5174 }`
fn extract_port_key_value(content: &str) -> Option<String> {
    port_key_value_re()
        .captures(content)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

/// 计算 dir 相对 root 的展示路径。
///
/// 能正常求相对路径则返回之（用 `/` 连接，跨平台一致）；若 dir 即为 root 或
/// 求相对路径失败（跨盘符等），退化为 dir 的文件名。
fn relpath(root: &Path, dir: &Path) -> String {
    match dir.strip_prefix(root) {
        Ok(rel) => {
            let s = rel.to_string_lossy().to_string();
            if s.is_empty() {
                // dir == root：用目录名展示
                dir.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| s)
            } else {
                // 统一用 '/' 分隔，避免 Windows 反斜杠
                s.replace('\\', "/")
            }
        }
        Err(_) => dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.to_string_lossy().to_string()),
    }
}

/// 一次性预处理 pom 内容：去注释 + 删首个 <parent> 块，供所有 pom 解析复用。
fn strip_pom(content: &str) -> String {
    remove_tag_block(&strip_xml_comments(content), "parent")
}

/// 判断已预处理的 pom 是否为聚合器（packaging=pom）。
fn is_pom_aggregator(stripped: &str) -> bool {
    if let Some(start) = stripped.find("<packaging>") {
        let rest = &stripped[start + "<packaging>".len()..];
        if let Some(end) = rest.find("</packaging>") {
            let val = rest[..end].trim();
            return val.eq_ignore_ascii_case("pom");
        }
    }
    false
}

/// 判断 Maven 项目是否真的含 main 入口方法。
///
/// 扫描 `src/main/java/` 下所有 `.java` 文件，查找标准 main 签名：
/// `public static void main(String[] args)`。这是比 pom 打包插件更可靠的
/// 「可启动模块」判定——类库/common 模块没有 main 方法，自然被过滤。
///
/// 匹配策略（对换行/空格/参数名宽松，避免漏判正常代码）：
/// - 先用正则去掉单行/多行注释，避免误命中注释里的 main
/// - 再匹配 `public static void main ( String [] <name> )`（void/static 顺序、
///   参数名、空格弹性）
fn has_main_class(dir: &Path) -> bool {
    let java_root = dir.join("src/main/java");
    let mut stack = vec![java_root.clone()];
    while let Some(cur) = stack.pop() {
        let entries = match std::fs::read_dir(&cur) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            let path = entry.path();
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() && path.extension().and_then(|e| e.to_str()) == Some("java") {
                if let Ok(src) = std::fs::read_to_string(&path) {
                    if file_has_main(&src) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 检测单个 Java 源文件内容是否含 main 方法。
fn file_has_main(src: &str) -> bool {
    // 快速路径：先对原始内容匹配（注释里恰好含 main 签名的情况极少），
    // 命中再脱注释二次确认，避免每个非 main 文件都做全文字符串分配。
    if !main_re().is_match(src) {
        return false;
    }
    let cleaned = strip_java_comments(src);
    main_re().is_match(&cleaned)
}

/// 剥离 Java 注释（单行 `//` 和多行 `/* */`），避免误命中注释内的 main。
fn strip_java_comments(src: &str) -> String {
    java_comment_re().replace_all(src, "").to_string()
}

fn extract_pom_artifact_id(stripped: &str) -> Option<String> {
    extract_first_tag(stripped, "artifactId")
}

/// 从已预处理的 pom 坐标组装 jar 名：artifactId-version.jar（finalName 优先）。
fn build_coord_jar_name(stripped: &str) -> Option<String> {
    let artifact = extract_first_tag(stripped, "artifactId")?;
    let version = extract_first_tag(stripped, "version");
    if let Some(fn_) = extract_first_tag(stripped, "finalName") {
        let fn_ = fn_.trim();
        if !fn_.is_empty() {
            return Some(format!("{fn_}.jar"));
        }
    }
    match version {
        Some(v) => Some(format!("{}-{}.jar", artifact.trim(), v.trim())),
        None => Some(format!("{}.jar", artifact.trim())),
    }
}

/// 扫 target 目录下的 jar（排除 sources/javadoc/classifier），返回文件名。
fn scan_target_jar(dir: &Path) -> Option<String> {
    let target = dir.join("target");
    let entries = std::fs::read_dir(&target).ok()?;
    let mut jars: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jar") {
            continue;
        }
        let lower = name.to_lowercase();
        if lower.ends_with("-sources.jar") || lower.ends_with("-javadoc.jar") {
            continue;
        }
        jars.push(name);
    }
    // 优先选不含 -sources/-javadoc 的「主」jar；若多个，取最短名（通常是 app.jar / xxx.jar 主产物）
    jars.sort_by_key(|s| s.len());
    jars.into_iter().next()
}

/// 取 XML 中首个 <tag>...</tag> 文本（不处理嵌套同名标签）。
fn extract_first_tag(s: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = s.find(&open)?;
    let rest = &s[start + open.len()..];
    let end = rest.find(&close)?;
    Some(rest[..end].trim().to_string())
}

/// 删除 XML 中首个 <tag>...</tag> 块（含内容），返回新字符串。
fn remove_tag_block(s: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let Some(start) = s.find(&open) else {
        return s.to_string();
    };
    let rest = &s[start..];
    let Some(rel_end) = rest.find(&close) else {
        return s.to_string();
    };
    let end = start + rel_end + close.len();
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..start]);
    out.push_str(&s[end..]);
    out
}

/// 剥离 XML 注释 <!-- ... -->。
fn strip_xml_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 4..];
        match after.find("-->") {
            Some(end) => rest = &after[end + 3..],
            None => {
                // 未闭合注释，丢弃剩余
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// 解析 properties 文件中的 server.port。
fn parse_properties_port(content: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == "server.port" {
                let v = v.trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

/// 解析 yml/yaml 中 server.port（简易缩进匹配，不引入 yaml 库）。
///
/// 逻辑：找到顶层 `server:` 块（缩进为 0），在该块的子层级里找 `port:` 键。
/// 健壮性：用「下一行缩进 ≤ server 缩进即视为块结束」来判断块边界，能正确处理
/// server 块出现在文档任意位置、其后还有其他顶层块的情况。
fn parse_yml_port(content: &str) -> Option<String> {
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && trimmed == "server:" {
            // 在 server 块内查找 port
            while let Some(&l) = lines.peek() {
                let lt = l.trim();
                if lt.starts_with('#') || lt.is_empty() {
                    lines.next();
                    continue;
                }
                let l_indent = l.len() - l.trim_start().len();
                // 缩进回到 server 同级或更上层 → server 块结束
                if l_indent <= indent {
                    break;
                }
                lines.next();
                if let Some((k, v)) = lt.split_once(':') {
                    if k.trim() == "port" {
                        let v = v.trim().trim_matches('"').trim_matches('\'');
                        if !v.is_empty() {
                            return Some(v.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

// ===== 单测 =====

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// 构造一个目录树辅助：在 tmp 下创建子目录并写入文件。
    fn write(rel: &Path, content: &str) {
        if let Some(parent) = rel.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(rel, content).unwrap();
    }

    #[test]
    fn aggregator_pom_detected_as_container() {
        let pom = r#"<?xml version="1.0"?>
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>parent</artifactId>
  <version>1.0</version>
  <packaging>pom</packaging>
  <modules>
    <module>svc-a</module>
  </modules>
</project>"#;
        assert!(is_pom_aggregator(&strip_pom(pom)));
    }

    #[test]
    fn non_aggregator_pom_not_container() {
        let pom = r#"<project>
  <artifactId>app</artifactId>
  <packaging>jar</packaging>
</project>"#;
        assert!(!is_pom_aggregator(&strip_pom(pom)));
    }

    #[test]
    fn artifact_id_skips_parent_block() {
        let pom = r#"<project>
  <parent>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.2.0</version>
  </parent>
  <artifactId>hr-service</artifactId>
  <version>0.0.1-SNAPSHOT</version>
</project>"#;
        assert_eq!(
            extract_pom_artifact_id(&strip_pom(pom)).as_deref(),
            Some("hr-service")
        );
    }

    #[test]
    fn coord_jar_name_from_artifact_version() {
        let pom = r#"<project>
  <artifactId>hr-service</artifactId>
  <version>0.0.1-SNAPSHOT</version>
</project>"#;
        assert_eq!(
            build_coord_jar_name(&strip_pom(pom)).as_deref(),
            Some("hr-service-0.0.1-SNAPSHOT.jar")
        );
    }

    #[test]
    fn coord_jar_name_with_finalname() {
        let pom = r#"<project>
  <artifactId>hr</artifactId>
  <version>1.0</version>
  <build><finalName>app</finalName></build>
</project>"#;
        assert_eq!(
            build_coord_jar_name(&strip_pom(pom)).as_deref(),
            Some("app.jar")
        );
    }

    #[test]
    fn vue_config_port_from_env_fallback() {
        // `const port = process.env.port || 8188`
        let content = "const port = process.env.port || 8188\nmodule.exports = { devServer: { port } };\n";
        assert_eq!(extract_port_from_config(content).as_deref(), Some("8188"));
    }

    #[test]
    fn vue_config_port_from_chained_or() {
        // 多级 || 链，取最后一个
        let content =
            "const PORT = process.env.port || process.env.npm_config_port || 9100;\n";
        assert_eq!(extract_fallback_port_after_or(content).as_deref(), Some("9100"));
    }

    #[test]
    fn vue_config_port_from_key_value() {
        // devServer: { port: 8888 }
        let content = "module.exports = {\n  devServer: {\n    port: 8888,\n    open: true\n  }\n};\n";
        assert_eq!(extract_port_key_value(content).as_deref(), Some("8888"));
    }

    #[test]
    fn vue_config_no_port_returns_none() {
        let content = "module.exports = { publicPath: '/' };\n";
        assert!(extract_port_from_config(content).is_none());
    }

    #[test]
    fn vite_config_port_from_server_block() {
        // server: { port: 5174 }
        let content = "export default defineConfig({\n  server: {\n    port: 5174,\n    host: true\n  }\n});\n";
        assert_eq!(extract_port_key_value(content).as_deref(), Some("5174"));
    }

    #[tokio::test]
    async fn scan_node_vue_cli_reads_config_port() {
        // vue-cli 项目：vue.config.js 里 port=8188，应提取 8188 而非默认 8080
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("web/package.json"),
            r#"{"name":"web","scripts":{"dev":"vue-cli-service serve"},"devDependencies":{"@vue/cli-service":"^5.0.0"}}"#,
        );
        write(
            &root.join("web/vue.config.js"),
            "const port = process.env.port || 8188\nmodule.exports = { devServer: { port } };\n",
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].expected_ports, vec!["8188".to_string()]);
    }

    #[tokio::test]
    async fn scan_node_vite_reads_config_port() {
        // vite 项目：vite.config.js 里 port=5174，应提取 5174 而非默认 5173
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("app/package.json"),
            r#"{"name":"app","scripts":{"dev":"vite"},"devDependencies":{"vite":"^5.0.0"}}"#,
        );
        write(
            &root.join("app/vite.config.js"),
            "import { defineConfig } from 'vite'\nexport default defineConfig({ server: { port: 5174 } })\n",
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].expected_ports, vec!["5174".to_string()]);
    }

    #[test]
    fn properties_port_parsed() {
        let content = "server.port=9090\nspring.application.name=demo\n";
        assert_eq!(parse_properties_port(content).as_deref(), Some("9090"));
    }

    #[test]
    fn properties_port_ignores_comments() {
        let content = "# server.port=80\nserver.port=8081\n";
        assert_eq!(parse_properties_port(content).as_deref(), Some("8081"));
    }

    #[test]
    fn yml_port_parsed() {
        let content = "server:\n  port: 8443\nspring:\n  application:\n    name: demo\n";
        assert_eq!(parse_yml_port(content).as_deref(), Some("8443"));
    }

    #[test]
    fn yml_port_parsed_when_server_in_middle() {
        // server 块出现在文档中间，其后还有其他顶层块
        let content = "spring:\n  application:\n    name: demo\nserver:\n  port: 9000\nlogging:\n  level: info\n";
        assert_eq!(parse_yml_port(content).as_deref(), Some("9000"));
    }

    #[test]
    fn yml_port_returns_none_when_no_server() {
        let content = "spring:\n  application:\n    name: demo\n";
        assert!(parse_yml_port(content).is_none());
    }

    #[test]
    fn main_method_detected_in_source() {
        let src = r#"package com.example;
public class App {
    public static void main(String[] args) {
        System.out.println("hello");
    }
}"#;
        assert!(file_has_main(src));
    }

    #[test]
    fn main_method_with_flexible_spacing() {
        // 换行/多空格/不同参数名
        let src = "public static   void\n  main (  String  [ ]  myArgs ) { }";
        assert!(file_has_main(src));
    }

    #[test]
    fn main_method_in_comment_ignored() {
        // 注释里的假 main 不应命中
        let src = r#"// public static void main(String[] args)
/* public static void main(String[] args) */
public class Util {}"#;
        assert!(!file_has_main(src));
    }

    #[test]
    fn library_without_main_not_matched() {
        let src = r#"package com.example;
public class Helper {
    public String greet() { return "hi"; }
}"#;
        assert!(!file_has_main(src));
    }

    #[test]
    fn spring_boot_pom_produces_three_schemes() {
        let schemes =
            DetectService::build_maven_schemes(true, &Some("app.jar".to_string()), "D:/code/repo", "svc");
        assert_eq!(schemes.len(), 3);
        // [0] 开发模式（默认）— -f 定位模块 + workingDirectory 让 fork JVM 在根目录运行
        assert!(schemes[0].recommended);
        assert!(schemes[0]
            .start_cmd
            .contains("mvn -f svc/pom.xml spring-boot:run"));
        assert!(schemes[0]
            .start_cmd
            .contains("-Dspring-boot.run.workingDirectory"));
        assert!(schemes[0].build_cmd.is_none());
        // [1] 开发模式（内嵌运行）— fork=false 规避 Windows error=206；不带 workingDirectory（fork=false 下无效）
        assert!(!schemes[1].recommended);
        assert!(schemes[1]
            .start_cmd
            .contains("mvn -f svc/pom.xml spring-boot:run -Dspring-boot.run.fork=false"));
        assert!(!schemes[1]
            .start_cmd
            .contains("workingDirectory"));
        assert!(schemes[1].build_cmd.is_none());
        // [2] 打包运行模式 — jar 路径含模块相对路径（svc/target/）
        assert!(schemes[2]
            .start_cmd
            .contains("java -jar svc/target/app.jar"));
        assert_eq!(
            schemes[2].build_cmd.as_deref(),
            Some("mvn clean package -DskipTests")
        );
    }

    #[test]
    fn plain_jar_pom_produces_one_scheme() {
        let schemes = DetectService::build_maven_schemes(false, &None, "D:/code/repo", "cli");
        assert_eq!(schemes.len(), 1);
        assert!(schemes[0].recommended);
        // jar 路径含模块相对路径
        assert!(schemes[0].start_cmd.contains("java -jar cli/target/"));
        assert!(schemes[0].build_cmd.is_some());
    }

    #[tokio::test]
    async fn scan_finds_node_and_springboot() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        // Node 项目：vite
        let pkg = root.join("frontend/package.json");
        write(
            &pkg,
            r#"{"name":"web","scripts":{"dev":"vite","build":"vite build"},"dependencies":{"vite":"^5.0.0"}}"#,
        );

        // SpringBoot 项目
        let pom = root.join("backend/hr/pom.xml");
        write(
            &pom,
            r#"<project>
  <parent>
    <groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.2.0</version>
  </parent>
  <artifactId>hr</artifactId>
  <version>0.0.1-SNAPSHOT</version>
</project>"#,
        );
        // 端口配置
        write(
            &root.join("backend/hr/src/main/resources/application.properties"),
            "server.port=8088\n",
        );
        // main 入口（启动模块硬条件）
        write(
            &root.join("backend/hr/src/main/java/com/example/HrApplication.java"),
            r#"package com.example;
import org.springframework.boot.SpringApplication;
public class HrApplication {
    public static void main(String[] args) {
        SpringApplication.run(HrApplication.class, args);
    }
}"#,
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        // 2 个项目
        assert_eq!(detected.len(), 2);
        // 按 rel_path 排序：backend/hr < frontend
        let back = &detected[0];
        let front = &detected[1];
        assert_eq!(back.r#type, ProjectType::Springboot);
        assert_eq!(back.name, "hr");
        assert_eq!(back.expected_ports, vec!["8088".to_string()]);
        assert_eq!(back.schemes.len(), 3);
        assert_eq!(front.r#type, ProjectType::Node);
        assert_eq!(front.name, "web");
        assert_eq!(front.expected_ports, vec!["5173".to_string()]);
        assert_eq!(front.schemes[0].start_cmd, "npm run dev");
    }

    #[tokio::test]
    async fn scan_skips_node_modules_and_target() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        // 一个真实 Node 项目
        write(
            &root.join("app/package.json"),
            r#"{"name":"app","scripts":{"dev":"vite"}}"#,
        );
        // node_modules 里伪装一个假 package.json，不应被检测
        write(
            &root.join("app/node_modules/fake/package.json"),
            r#"{"name":"fake","scripts":{"dev":"x"}}"#,
        );
        // target 里伪装 pom.xml，不应被检测
        write(
            &root.join("app/target/sub/pom.xml"),
            r#"<project><artifactId>x</artifactId></project>"#,
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name, "app");
    }

    #[tokio::test]
    async fn scan_monorepo_workspace_descends() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        // 根 workspace
        write(
            &root.join("package.json"),
            r#"{"name":"mono","private":true,"workspaces":["packages/*"]}"#,
        );
        // 子包
        write(
            &root.join("packages/web/package.json"),
            r#"{"name":"web","scripts":{"dev":"vite"}}"#,
        );
        write(
            &root.join("packages/api/package.json"),
            r#"{"name":"api","scripts":{"start":"node server.js"}}"#,
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        // 根 workspace 不入结果，2 个子包入结果
        assert_eq!(detected.len(), 2);
        assert!(detected.iter().any(|d| d.name == "web"));
        assert!(detected.iter().any(|d| d.name == "api"));
    }

    #[tokio::test]
    async fn scan_aggregator_pom_descends_to_modules() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        // 聚合器根
        write(
            &root.join("pom.xml"),
            r#"<project>
  <artifactId>parent</artifactId>
  <packaging>pom</packaging>
  <modules><module>svc</module></modules>
</project>"#,
        );
        // 子模块 SpringBoot
        write(
            &root.join("svc/pom.xml"),
            r#"<project>
  <artifactId>svc</artifactId>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.2.0</version></parent>
</project>"#,
        );
        // 子模块的 main + 端口
        write_springboot_main(&root.join("svc"), "com/example", "SvcApplication");
        write(
            &root.join("svc/src/main/resources/application.properties"),
            "server.port=8080\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].r#type, ProjectType::Springboot);
    }

    #[tokio::test]
    async fn scan_node_without_scripts_is_skipped() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("lib/package.json"),
            r#"{"name":"lib","description":"a lib without scripts"}"#,
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert!(detected.is_empty());
    }

    #[tokio::test]
    async fn scan_nonexistent_path_errors() {
        let err = DetectService::scan("/no/such/path/xyz").await.unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
    }

    /// 辅助：写入一个标准 SpringBoot 启动类（含 main）
    fn write_springboot_main(root: &Path, rel_pkg: &str, class: &str) {
        let path = root.join(format!("src/main/java/{rel_pkg}/{class}.java"));
        write(
            &path,
            &format!(
                r#"package {pkg};
import org.springframework.boot.SpringApplication;
public class {cls} {{
    public static void main(String[] args) {{
        SpringApplication.run({cls}.class, args);
    }}
}}"#,
                pkg = rel_pkg.replace('/', "."),
                cls = class
            ),
        );
    }

    #[tokio::test]
    async fn scan_jar_name_from_target_overrides_coord() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("svc/pom.xml"),
            r#"<project>
  <artifactId>svc</artifactId>
  <version>1.0</version>
</project>"#,
        );
        // main 入口 + 端口（启动模块双硬条件）
        write_springboot_main(&root.join("svc"), "com/example", "SvcApplication");
        write(
            &root.join("svc/src/main/resources/application.properties"),
            "server.port=9000\n",
        );
        // target 里已有构建产物
        fs::create_dir_all(root.join("svc/target")).unwrap();
        fs::write(root.join("svc/target/svc-1.0.jar"), b"").unwrap();

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        // 主 jar 名应来自 target 扫描
        assert!(detected[0].schemes[0]
            .start_cmd
            .contains("target/svc-1.0.jar"));
    }

    #[tokio::test]
    async fn scan_filters_library_without_main() {
        // 无 main 方法 → 类库，过滤
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("common/pom.xml"),
            r#"<project>
  <artifactId>common-utils</artifactId>
  <version>1.0</version>
</project>"#,
        );
        // 有端口但无 main
        write(
            &root.join("common/src/main/resources/application.properties"),
            "server.port=8080\n",
        );
        write(
            &root.join("common/src/main/java/com/example/Util.java"),
            r#"package com.example;
public class Util {
    public String hello() { return "hi"; }
}"#,
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert!(detected.is_empty());
    }

    #[tokio::test]
    async fn scan_filters_project_without_port() {
        // 有 main 但无端口配置 → 无法健康检查，过滤
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("svc/pom.xml"),
            r#"<project>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.2.0</version></parent>
  <artifactId>svc</artifactId>
</project>"#,
        );
        write_springboot_main(root, "com/example", "SvcApplication");
        // 故意不写 application 配置 → 无端口
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert!(detected.is_empty());
    }

    #[tokio::test]
    async fn scan_keeps_executable_jar_project() {
        // 有 main + 有端口 → 普通可执行 jar 项目入结果
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("cli/pom.xml"),
            r#"<project>
  <artifactId>mycli</artifactId>
  <version>2.0</version>
</project>"#,
        );
        write(
            &root.join("cli/src/main/java/com/app/Main.java"),
            r#"package com.app;
public class Main {
    public static void main(String[] args) {
        Server.start(7000);
    }
}"#,
        );
        write(
            &root.join("cli/src/main/resources/application.properties"),
            "server.port=7000\n",
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].r#type, ProjectType::JavaJar);
        assert_eq!(detected[0].expected_ports, vec!["7000".to_string()]);
        assert!(detected[0].schemes[0]
            .start_cmd
            .contains("java -jar cli/target/"));
    }

    #[tokio::test]
    async fn scan_springboot_with_port_detected() {
        // SpringBoot：有 main + 显式端口 → 入结果
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("svc/pom.xml"),
            r#"<project>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.2.0</version></parent>
  <artifactId>svc</artifactId>
</project>"#,
        );
        write_springboot_main(&root.join("svc"), "com/example", "SvcApplication");
        write(
            &root.join("svc/src/main/resources/application.properties"),
            "server.port=8088\n",
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].expected_ports, vec!["8088".to_string()]);
    }

    #[tokio::test]
    async fn scan_reads_multi_profile_ports() {
        // application.yml + application-dev.yml 含不同端口，都应被提取
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("svc/pom.xml"),
            r#"<project>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.2.0</version></parent>
  <artifactId>svc</artifactId>
</project>"#,
        );
        write_springboot_main(&root.join("svc"), "com/example", "SvcApplication");
        write(
            &root.join("svc/src/main/resources/application.yml"),
            "server:\n  port: 8080\n",
        );
        write(
            &root.join("svc/src/main/resources/application-dev.yml"),
            "server:\n  port: 18080\n",
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        // 两个 profile 的端口都应出现
        assert!(detected[0].expected_ports.contains(&"8080".to_string()));
        assert!(detected[0].expected_ports.contains(&"18080".to_string()));
    }
}
