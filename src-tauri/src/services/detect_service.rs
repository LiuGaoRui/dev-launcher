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
///
/// 分两类：
/// - 依赖/产物目录：node_modules、target、dist 等，体积大且从不含项目源码
/// - 数据/临时目录：data、repos、logs、tmp 等，常被克隆仓库或运行时产物污染，
///   如「审查Agent」项目的 data/repos/ 下有 82 个无关 hanxiinfotech 模块 pom.xml
const SKIP_DIRS: &[&str] = &[
    // 依赖与构建产物
    "node_modules",
    "target",
    "dist",
    "build",
    ".gradle",
    ".next",
    ".nuxt",
    "__pycache__",
    "venv",
    ".venv",
    "coverage",
    // IDE / VCS
    ".git",
    ".idea",
    ".vscode",
    ".cache",
    // 数据 / 日志 / 临时（防止克隆仓库、运行时产物等污染扫描结果）
    "data",
    "repos",
    "logs",
    "log",
    "tmp",
    "temp",
    "bin",
    "out",
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
                    // 不 return：继续下钻子目录，以便发现同项目下的前端（如 web/package.json）
                    // 等其他类型子项目。SKIP_DIRS + MAX_DEPTH 已限制扫描范围。
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
                        // 不 return：继续下钻子目录，以便发现同项目下的后端（如 server/）
                        // 等其他类型子项目。SKIP_DIRS + MAX_DEPTH 已限制扫描范围。
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

    /// 判断 package.json 是否为 monorepo 容器根（不入结果，继续下钻找子项目）。
    ///
    /// 两种形态：
    /// 1. 标准 workspace：声明了 `workspaces` 字段（npm/yarn workspaces）
    /// 2. 协调脚本根：无 workspaces，但所有启动脚本（dev/start/serve 等）都以
    ///    `cd <子目录> &&` 开头——这是 monorepo 用脚本转发到子项目的常见写法，
    ///    根本身不提供服务，扫出会和子项目重复。
    fn is_node_workspace(pkg: &serde_json::Value) -> bool {
        // 形态 1：显式 workspaces 字段
        if pkg.get("workspaces").is_some() {
            return true;
        }
        // 形态 2：协调脚本根——所有脚本都 cd 到子目录
        let Some(scripts) = pkg.get("scripts").and_then(|s| s.as_object()) else {
            return false;
        };
        // 关注会启动服务的脚本键
        let launch_keys = ["dev", "start", "serve", "web"];
        let launch_scripts: Vec<&str> = launch_keys
            .iter()
            .filter_map(|k| scripts.get(*k).and_then(|v| v.as_str()))
            .collect();
        // 至少要有 2 个这样的脚本（单个 cd 可能是巧合，多个则强烈暗示协调脚本）
        if launch_scripts.len() < 2 {
            return false;
        }
        // 全部以 `cd <dir> &&` 开头才认定（若混有非 cd 脚本，说明根自身也提供服务）
        launch_scripts
            .iter()
            .all(|s| coord_script_re().is_match(s))
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
            return vec!["4200".to_string()];
        } else if has("next") || has("react-scripts") || has("nuxt") {
            return vec!["3000".to_string()];
        }

        // 兜底：扫描源码中的监听端口（Express/Fastify/Koa 等纯后端）
        // 匹配 process.env.PORT || 3001、app.listen(3001 等常见写法
        if let Some(p) = scan_source_port(dir) {
            return vec![p];
        }

        Vec::new()
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
        // pom 传入：本地无 resources 时兜底扫 pom <resource><directory> 引用的兄弟模块配置
        let expected_ports = Self::read_java_port(dir, &stripped);
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
        // JAVA_HOME：从启动脚本提取（多 JDK 项目需要，如审查Agent 用 Java 21）
        let java_home = Self::detect_java_home(dir);
        // 展示用相对路径（dir==root 时为目录名）
        let mod_rel = relpath(root, dir);
        // 命令用相对路径（dir==root 时为空串，避免 -f 审查Agent/pom.xml 误拼 + 中文乱码）
        let cmd_rel = relpath_cmd(root, dir);
        let root_str = root.to_string_lossy().to_string();
        // 父聚合器 pom（多模块 reactor）：有则开发模式走 -pl <module> -am 跨模块构建依赖
        // 注意用原始 content（含 <parent>），stripped 已删 parent 块
        let aggregator = find_aggregator_pom(dir, &content);
        let aggregator_rel = aggregator
            .as_ref()
            .and_then(|p| {
                let rel = relpath_cmd(root, p);
                if rel.is_empty() { None } else { Some(rel) }
            });
        // 模块目录名（用于 mvn -pl）：dir==root 时为空串（无聚合器场景不用）
        let module_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // spring-boot ZIP layout：打包运行需 -Dloader.path=./lib
        let zip_layout = is_zip_layout(&stripped);
        // 作者预构建的 release/ 目录（仅 ZIP layout 时检测；依赖更完整）
        let release_rel = if zip_layout {
            find_release_dir(dir).and_then(|p| {
                let rel = relpath_cmd(root, &p);
                if rel.is_empty() { None } else { Some(rel) }
            })
        } else {
            None
        };

        let schemes = Self::build_maven_schemes(
            &jar_name,
            &cmd_rel,
            java_home.as_deref(),
            aggregator_rel.as_deref(),
            &module_name,
            zip_layout,
            release_rel.as_deref(),
        );

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

    /// 从项目启动脚本提取 JAVA_HOME（多 JDK 项目用，如审查Agent 需 Java 21）。
    ///
    /// 扫描 dir 下的 `*.bat` / `*.cmd` / `*.sh` / `*.ps1`，正则匹配
    /// `set JAVA_HOME=<路径>` / `JAVA_HOME=<路径>`（不区分大小写）。
    /// 返回首个命中路径（去引号、去尾部注释）。无脚本或无匹配返回 None。
    ///
    /// 设计动机：项目作者在 start.bat/build.bat 里声明的 JAVA_HOME 是「正确 JDK 在哪」
    /// 的权威信息；生成的命令注入它，可避免系统 PATH 指向错误 JDK（如 Java 8）。
    fn detect_java_home(dir: &Path) -> Option<String> {
        let entries = std::fs::read_dir(dir).ok()?;
        // 优先扫 .bat/.cmd（Windows 主流），再 .sh/.ps1
        let mut scripts: Vec<PathBuf> = Vec::new();
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if matches!(
                        ext.to_lowercase().as_str(),
                        "bat" | "cmd" | "sh" | "ps1"
                    ) {
                        scripts.push(p);
                    }
                }
            }
        }
        // start.* / run.* / build.* 优先（更可能是启动脚本），其余按字典序
        scripts.sort_by(|a, b| {
            let prio = |n: &str| match n {
                "start" => 0,
                "run" => 1,
                "build" => 2,
                _ => 9,
            };
            let an = a
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let bn = b
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            prio(&an).cmp(&prio(&bn)).then_with(|| an.cmp(&bn))
        });

        let re = java_home_re();
        for script in &scripts {
            let Ok(content) = std::fs::read_to_string(script) else {
                continue;
            };
            for line in content.lines() {
                if let Some(c) = re.captures(line) {
                    if let Some(m) = c.get(1) {
                        // 去引号、去尾部注释/分号（bat 的 `set` 行尾一般干净，防御性处理）
                        let v = m
                            .as_str()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .trim();
                        // 去掉可能的行内注释（rem :: #）
                        let v = v
                            .split_whitespace()
                            .next()
                            .unwrap_or(v)
                            .to_string();
                        if !v.is_empty() {
                            return Some(v);
                        }
                    }
                }
            }
        }
        None
    }

    /// 读 Java 应用端口：扫描所有 application 配置文件（含多 profile），
    /// 返回发现的全部端口（去重）。
    ///
    /// 覆盖：`application.properties` / `application.yml` / `application.yaml`
    /// 及 `application-{profile}.properties/.yml/.yaml`。
    ///
    /// `pom` 为已脱注释/删 parent 的 pom 文本：当模块本地无 `src/main/resources`
    /// 时（如 hmsoft-boot-jar 把配置放在兄弟模块、经 pom `<resource><directory>`
    /// 引入），兜底扫 pom 声明的资源目录，避免误判为无端口的类库。
    fn read_java_port(dir: &Path, pom: &str) -> Vec<String> {
        // 1. 本地 src/main/resources
        let local = dir.join("src/main/resources");
        let mut ports = scan_application_ports(&local);

        // 2. 本地无端口 → 兜底扫 pom <resource><directory> 引用的兄弟模块配置
        if ports.is_empty() {
            for res_dir in extract_pom_resource_dirs(pom, dir) {
                if res_dir == local {
                    continue;
                }
                let extra = scan_application_ports(&res_dir);
                if !extra.is_empty() {
                    ports = extra;
                    break;
                }
            }
        }
        ports
    }

    /// 生成 Maven 项目的启动方案。
    ///
    /// 分步执行：start_cmd 只 `java -jar`（秒级启动），build_cmd = `mvn package`。
    /// 改代码后手动：点「构建」→ 点「停止」→ 点「启动」。
    /// 不再用 spring-boot:run（多模块项目传递依赖解析不可靠，会 NoClassDefFoundError）。
    ///
    /// - `jar_name`：推断的 jar 文件名
    /// - `mod_rel`：模块相对根目录的**命令路径**（如 "backend/hr"）；模块即扫描根时为空串
    /// - `java_home`：从启动脚本提取的 JAVA_HOME（多 JDK 项目用）；None 则用系统 PATH
    /// - `aggregator_rel`：父聚合器 pom 相对 root 的路径（多模块 reactor）；None 表示无聚合器
    /// - `module_name`：模块目录名（用于 mvn `-pl`，仅在聚合器场景生效）
    /// - `zip_layout`：spring-boot 是否 ZIP layout（打包运行需 `-Dloader.path=./lib`）
    /// - `release_rel`：作者预构建的 `release/` 目录相对 root 的路径；有则优先从 release/ 运行
    #[allow(clippy::too_many_arguments)]
    fn build_maven_schemes(
        jar_name: &Option<String>,
        mod_rel: &str,
        java_home: Option<&str>,
        aggregator_rel: Option<&str>,
        module_name: &str,
        zip_layout: bool,
        release_rel: Option<&str>,
    ) -> Vec<LaunchScheme> {
        // 有 JAVA_HOME 时：mvn 前加 `set "JAVA_HOME=..." && `；java 用全路径
        let mvn_prefix = match java_home {
            Some(jh) => format!("set \"JAVA_HOME={jh}\" && "),
            None => String::new(),
        };
        let java_bin = match java_home {
            Some(jh) => format!("\"{jh}\\bin\\java.exe\""),
            None => "java".to_string(),
        };

        // build_cmd = mvn package（改代码后手动点「构建」执行）
        let f_param = if let Some(agg) = aggregator_rel {
            if !module_name.is_empty() {
                let agg_quoted = if agg.contains(' ') {
                    format!("\"{agg}\"")
                } else {
                    agg.to_string()
                };
                format!("-f {agg_quoted} -pl {module_name} -am ")
            } else {
                "-f pom.xml ".to_string()
            }
        } else if mod_rel.is_empty() {
            "-f pom.xml ".to_string()
        } else if mod_rel.contains(' ') {
            format!("-f \"{mod_rel}/pom.xml\" ")
        } else {
            format!("-f {mod_rel}/pom.xml ")
        };
        let build_cmd = Some(format!("{mvn_prefix}mvn {f_param}clean package -DskipTests"));

        // start_cmd = 只 java -jar（秒级启动，不含打包）
        // - ZIP layout：优先 release/（完整依赖），无则 target/（+ loader.path=./lib）
        // - 普通 layout：target/<jar>，路径从 root 相对（含空格加引号）
        let jar_name_str = jar_name.as_deref().unwrap_or("app.jar");
        let start_cmd = if zip_layout {
            if let Some(rel) = release_rel {
                let cd = if rel.contains(' ') {
                    format!("cd \"{rel}\" && ")
                } else {
                    format!("cd {rel} && ")
                };
                format!("{cd}{java_bin} -Dloader.path=./lib -jar {jar_name_str}")
            } else {
                let cd = if mod_rel.is_empty() {
                    String::new()
                } else if mod_rel.contains(' ') {
                    format!("cd \"{mod_rel}\" && ")
                } else {
                    format!("cd {mod_rel} && ")
                };
                format!("{cd}{java_bin} -Dloader.path=./lib -jar target/{jar_name_str}")
            }
        } else {
            let prefix = if mod_rel.is_empty() {
                "target/".to_string()
            } else {
                format!("{mod_rel}/target/")
            };
            if prefix.contains(' ') {
                format!("{java_bin} -jar \"{prefix}{jar_name_str}\"")
            } else {
                format!("{java_bin} -jar {prefix}{jar_name_str}")
            }
        };

        let desc = match jar_name {
            Some(_) => "运行已构建的 jar。改代码后请先点「构建」重新打包，再停止后重新启动",
            None => "运行已构建的 jar；jar 名未能自动识别，请按实际产物修正",
        };

        vec![LaunchScheme {
            label: "打包运行模式".to_string(),
            recommended: true,
            start_cmd,
            build_cmd,
            description: desc.to_string(),
        }]
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
// const/let/var port = <数字>（vue.config.js 常见：const port = 8199）
// 等号右边要求紧跟数字，故 `const port = process.env.port || 8199` 不会误命中
lazy_regex!(port_assign_re, r"(?i)(?:const|let|var)\s+port\s*=\s*(\d{2,5})\b");
lazy_regex!(main_re, r"public\s+static\s+void\s+main\s*\(\s*String\s*\[\s*\]\s*\w+\s*\)");
lazy_regex!(java_comment_re, r"//[^\n]*|/\*[\s\S]*?\*/");
// Node 后端源码端口：process.env.PORT || 3001 / '3001' / "3001"（兼容带引号字符串写法）
lazy_regex!(env_port_re, r#"process\.env\.PORT\s*\|\|\s*['"]?(\d{2,5})['"]?"#);
// Node 后端源码端口：app.listen(3001 / server.listen( 3000
lazy_regex!(listen_port_re, r"\.listen\s*\(\s*(\d{2,5})");
// 协调脚本：以 `cd <子目录> &&` 开头（monorepo 转发到子项目的常见写法）
lazy_regex!(coord_script_re, r"^\s*cd\s+\S+\s*&&");
// 启动脚本里的 JAVA_HOME 声明：set JAVA_HOME=... / JAVA_HOME=...（不区分大小写）
lazy_regex!(java_home_re, r#"(?i)^\s*(?:set\s+)?JAVA_HOME\s*=\s*(\S+)"#);

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

/// 从 JS/TS 配置文件文本中提取端口。三级兜底：
/// 1. `|| <port>` 链（取最后一个兜底值）
/// 2. `port: <数字>` 键值
/// 3. `const/let/var port = <数字>` 常量赋值（如 vue.config.js 的 `const port = 8199`）
fn extract_port_from_config(content: &str) -> Option<String> {
    extract_fallback_port_after_or(content)
        .or_else(|| extract_port_key_value(content))
        .or_else(|| extract_port_const_assignment(content))
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

/// 兜底：扫描 Node 后端源码（src/ 下）中的监听端口。
///
/// 适用于无框架特征（非 vite/vue/next）的纯后端（Express/Fastify/Koa）项目，
/// 其端口常硬编码在源码里。匹配两种写法：
/// - `process.env.PORT || 3001`（环境变量兜底常量）
/// - `app.listen(3001` / `server.listen( 3001`（直接传入数字）
///
/// 多文件按「index/main 优先 + 字典序」排序，取第一个命中，符合 Node 入口惯例。
fn scan_source_port(dir: &Path) -> Option<String> {
    let src = dir.join("src");
    let src_dir = if src.is_dir() {
        src
    } else {
        // 无 src/ 时退到项目根目录（部分项目源码直接在根目录）
        dir.to_path_buf()
    };

    // 收集 src/ 下（仅一层，避免深递归）的 js/ts/mjs/cjs 文件
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "js" | "ts" | "mjs" | "cjs") {
                        files.push(p);
                    }
                }
            }
        }
    }
    if files.is_empty() {
        return None;
    }

    // index.* / main.* 优先，其余按文件名字典序；保证稳定且符合入口惯例
    files.sort_by(|a, b| {
        let an = a
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let bn = b
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let prio = |n: &str| match n {
            "index" => 0,
            "main" => 1,
            "app" => 2,
            "server" => 3,
            _ => 9,
        };
        prio(&an)
            .cmp(&prio(&bn))
            .then_with(|| an.cmp(&bn))
    });

    let re_env_port = env_port_re();
    let re_listen = listen_port_re();
    for file in &files {
        if let Ok(content) = std::fs::read_to_string(file) {
            // 先匹配 process.env.PORT || N（语义最明确）
            if let Some(p) = re_env_port
                .captures(&content)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            {
                return Some(p);
            }
            // 再匹配 .listen( N
            if let Some(p) = re_listen
                .captures(&content)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
            {
                return Some(p);
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

/// 提取 `const/let/var port = <数字>` 常量赋值写法中的端口。
///
/// 例：`const port = 8199` / `const PORT = 8199`（变量随后被 `port: port` 引用，
/// 这种引用写法键值正则匹配不到，需直接取赋值处的数字）。
/// 取首个命中（一个配置文件只声明一次端口变量）。
fn extract_port_const_assignment(content: &str) -> Option<String> {
    port_assign_re()
        .captures(content)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

/// 计算 dir 相对 root 的展示路径。
///
/// dir == root 时返回目录名（如「审查Agent」）作为展示，而非空串。
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

/// 计算 dir 相对 root 的命令路径（用于 mvn -f / java -jar 等命令拼接）。
///
/// 与 [`relpath`] 区别：dir == root 时返回**空串**，调用方据此判断「模块即扫描根」，
/// 此时 cwd 已在根目录，不应拼 `<mod_rel>/pom.xml`（会变成 `审查Agent/审查Agent/pom.xml`），
/// 也不该把含中文/空格的目录名作为 -f 参数（经 cmd.exe 传递易乱码）。
fn relpath_cmd(root: &Path, dir: &Path) -> String {
    match dir.strip_prefix(root) {
        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
        Err(_) => String::new(),
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

/// 检测模块所属的父聚合器 pom（reactor 根）。
///
/// 多模块项目里，启动模块（如 hmsoft-boot-jar）的源码引用兄弟模块（hmsoft-boot）的类，
/// 单跑 `mvn -f <module>/pom.xml` 会因兄弟模块未 install 到本地仓库而编译失败。此时需让
/// mvn 指向**聚合器 pom** + `-pl <module> -am`，让 Maven 在 reactor 内构建依赖模块。
///
/// 判定（避免误命中碰巧在上级目录的 pom）：
/// 1. 模块 pom 声明了 `<parent>`（有父）
/// 2. `module_dir/../pom.xml` 存在
/// 3. 该 pom 是聚合器（`<packaging>pom</packaging>`）
/// 4. 该 pom 的 `<modules>` 列出本模块目录名 —— 确认是真正的 reactor 父
///
/// 命中返回聚合器 pom 的绝对路径；任一条件不满足返回 None。
fn find_aggregator_pom(module_dir: &Path, pom_content: &str) -> Option<PathBuf> {
    // 1. 必须有 <parent>（无 parent 的根 pom 不适用此机制）
    if !pom_content.contains("<parent>") {
        return None;
    }
    // 2. 上级目录的 pom.xml
    let parent_pom = module_dir.parent()?.join("pom.xml");
    if !parent_pom.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(&parent_pom).ok()?;
    let stripped = strip_pom(&content);
    // 3. 必须是聚合器（packaging=pom）
    if !is_pom_aggregator(&stripped) {
        return None;
    }
    // 4. <modules> 须列出本模块目录名
    let module_name = module_dir.file_name()?.to_string_lossy().into_owned();
    if !aggregator_includes_module(&stripped, &module_name) {
        return None;
    }
    Some(parent_pom)
}

/// 判断聚合器 pom 的 `<modules>` 是否列出指定模块名。
///
/// 匹配 `<module>name</module>`（name 前后容空格）。不区分 `modules` 块内是否有其他内容。
fn aggregator_includes_module(stripped_aggregator: &str, module_name: &str) -> bool {
    let Some(block) = extract_first_tag(stripped_aggregator, "modules") else {
        return false;
    };
    let mut found = false;
    for_each_child_tag(&block, "module", |m| {
        if m.trim() == module_name {
            found = true;
        }
    });
    found
}

/// 检测 pom 是否声明 spring-boot 打包插件 `<layout>ZIP</layout>`。
///
/// ZIP layout 的 jar 由 PropertiesLauncher 加载，须配合 `-Dloader.path=./lib`
/// 从外部 lib/ 目录加载依赖（非内嵌 fat-jar）。
fn is_zip_layout(stripped_pom: &str) -> bool {
    if let Some(layout) = extract_first_tag(stripped_pom, "layout") {
        return layout.trim().eq_ignore_ascii_case("ZIP");
    }
    false
}

/// 检测模块下是否存在作者预构建的运行时分发目录 `release/`。
///
/// ZIP layout 项目（如 hmsoft-boot-jar）的 `target/lib/`（copy-dependencies 生成）
/// 可能依赖不完整（缺少部分第三方 jar）。而 `release/` 是作者打包好的完整分发：
/// jar + 完整 lib/ + application.yml + license.lic，自包含可运行。
///
/// 判定：`dir/release/` 存在且至少含一个 jar 文件。命中返回该目录的绝对路径。
fn find_release_dir(dir: &Path) -> Option<PathBuf> {
    let release = dir.join("release");
    if !release.is_dir() {
        return None;
    }
    // 确认里面有 jar（而非空目录）
    let entries = std::fs::read_dir(&release).ok()?;
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            if entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map_or(false, |e| e.eq_ignore_ascii_case("jar"))
            {
                return Some(release);
            }
        }
    }
    None
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

/// 在 XML 块内逐个提取 `<tag>...</tag>` 文本，对每个命中调用 `f`。
///
/// 与 [`extract_first_tag`] 组合：先用它取外层块（如 `<modules>`），再用本函数
/// 遍历块内子标签（如 `<module>`）。不处理嵌套同名标签（此模块所有 pom 标签均为扁平结构）。
fn for_each_child_tag(block: &str, tag: &str, mut f: impl FnMut(&str)) {
    let close_tag = format!("</{tag}>");
    let mut rest = block;
    while let Some(content) = extract_first_tag(rest, tag) {
        f(&content);
        match rest.find(&close_tag) {
            Some(pos) => rest = &rest[pos + close_tag.len()..],
            None => break,
        }
    }
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

/// 扫描某个目录下所有 `application*` 配置文件，返回发现的全部端口（去重）。
///
/// 默认配置在前、profile 配置在后，按文件名排序保证多次扫描结果稳定。
/// properties 走 [`parse_properties_port`]，yml/yaml 走 [`parse_yml_port`]。
/// 目录不存在或无配置文件返回空 Vec。
fn scan_application_ports(res_dir: &Path) -> Vec<String> {
    let entries = match std::fs::read_dir(res_dir) {
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

/// 从已预处理的 pom 中提取 `<build><resources>` 里所有 `<directory>` 路径，
/// 按模块目录解析为绝对路径。
///
/// 适用于启动模块自身无 `src/main/resources`、把配置放在兄弟模块、经 pom
/// `<resource><directory>../hmsoft-boot/src/main/resources</directory>` 引入的场景。
/// 用字符串方式定位首个 `<resources>...</resources>` 块（与现有 pom 解析风格一致，
/// 不引入重型 XML 库）；块内取所有 `<directory>` 文本，相对模块目录解析。
/// 路径不存在或无法解析的条目自动跳过。
fn extract_pom_resource_dirs(pom: &str, module_dir: &Path) -> Vec<PathBuf> {
    let Some(block) = extract_first_tag(pom, "resources") else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    for_each_child_tag(&block, "directory", |d| {
        let d = d.trim();
        if !d.is_empty() {
            let resolved = module_dir.join(d);
            if resolved.is_dir() {
                dirs.push(resolved);
            }
        }
    });
    dirs
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
    fn zip_layout_detected_from_pom() {
        let pom = r#"<project>
  <artifactId>app</artifactId>
  <build>
    <plugins>
      <plugin>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-maven-plugin</artifactId>
        <configuration>
          <layout>ZIP</layout>
        </configuration>
      </plugin>
    </plugins>
  </build>
</project>"#;
        assert!(is_zip_layout(&strip_pom(pom)));
        // 小写 zip 也应识别
        let pom_lower = pom.replace("ZIP", "zip");
        assert!(is_zip_layout(&strip_pom(&pom_lower)));
    }

    #[test]
    fn non_zip_layout_not_detected() {
        let pom = r#"<project>
  <artifactId>app</artifactId>
  <build><finalName>app</finalName></build>
</project>"#;
        assert!(!is_zip_layout(&strip_pom(pom)));
    }

    #[tokio::test]
    async fn find_aggregator_pom_locates_reactor_parent() {
        // 多模块 reactor：父 pom 是聚合器且列出本模块 → 应找到它
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        // 父聚合器 pom
        write(
            &root.join("pom.xml"),
            r#"<project>
  <groupId>com.hmsoft</groupId>
  <artifactId>parent</artifactId>
  <version>7.1.0</version>
  <packaging>pom</packaging>
  <modules>
    <module>hmsoft-boot</module>
    <module>hmsoft-boot-jar</module>
  </modules>
</project>"#,
        );
        // 启动模块 pom（有 parent，自身在 hmsoft-boot-jar/）
        let module_pom = r#"<project>
  <artifactId>hmsoft-boot-jar</artifactId>
  <packaging>jar</packaging>
  <parent>
    <groupId>com.hmsoft</groupId>
    <artifactId>parent</artifactId>
    <version>7.1.0</version>
  </parent>
</project>"#;
        write(&root.join("hmsoft-boot-jar/pom.xml"), module_pom);
        let module_dir = root.join("hmsoft-boot-jar");
        let found = find_aggregator_pom(&module_dir, module_pom);
        assert_eq!(
            found.as_deref(),
            Some(root.join("pom.xml").as_path())
        );
    }

    #[tokio::test]
    async fn find_aggregator_pom_rejects_parent_not_listing_module() {
        // 父 pom 是聚合器但 modules 里没有本模块 → 不应误命中
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("pom.xml"),
            r#"<project>
  <artifactId>parent</artifactId>
  <packaging>pom</packaging>
  <modules><module>other-module</module></modules>
</project>"#,
        );
        let module_pom = r#"<project>
  <artifactId>hmsoft-boot-jar</artifactId>
  <parent><artifactId>parent</artifactId></parent>
</project>"#;
        write(&root.join("hmsoft-boot-jar/pom.xml"), module_pom);
        let module_dir = root.join("hmsoft-boot-jar");
        assert!(find_aggregator_pom(&module_dir, module_pom).is_none());
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
    fn vue_config_port_from_const_assignment() {
        // vue.config.js 常见写法：const port = 8199，随后 port: port 引用变量
        // 键值正则匹配 `port: port`（非数字）会失败，需走常量赋值兜底
        let content = "const port = 8199\nmodule.exports = { devServer: { port: port } };\n";
        assert_eq!(extract_port_from_config(content).as_deref(), Some("8199"));
    }

    #[test]
    fn vue_config_const_assignment_case_insensitive() {
        // const PORT = 8199（大写变量名，正则不区分大小写）
        let content = "const PORT = 8199\n";
        assert_eq!(extract_port_const_assignment(content).as_deref(), Some("8199"));
    }

    #[test]
    fn vue_config_const_port_with_env_fallback_not_mismatched() {
        // const port = process.env.port || 8199：等号右边非纯数字，常量赋值正则不应命中；
        // 但 fallback 正则会命中 || 8199，故整体仍应取 8199（验证常量正则不破坏既有逻辑）
        let content = "const port = process.env.port || 8199\n";
        assert_eq!(extract_port_const_assignment(content), None);
        assert_eq!(extract_port_from_config(content).as_deref(), Some("8199"));
    }

    #[tokio::test]
    async fn scan_node_vue_cli_const_port() {
        // vue-cli 项目：vue.config.js 里 `const port = 8199` + `port: port` 引用，
        // 应提取 8199 而非默认 8080（真实「HanXiInfotech oa vue」场景）
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("web/package.json"),
            r#"{"name":"web","scripts":{"dev":"vue-cli-service serve"},"devDependencies":{"@vue/cli-service":"~5.0.0"}}"#,
        );
        write(
            &root.join("web/vue.config.js"),
            "const port = 8199\nmodule.exports = { devServer: { port: port } };\n",
        );
        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].expected_ports, vec!["8199".to_string()]);
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
    fn maven_scheme_start_jar_build_package() {
        // start_cmd 只 java -jar；build_cmd = mvn package
        let schemes = DetectService::build_maven_schemes(
            &Some("app.jar".to_string()),
            "svc",
            None,
            None,
            "svc",
            false,
            None,
        );
        assert_eq!(schemes.len(), 1);
        assert!(schemes[0].recommended);
        // start_cmd 只有 java -jar（不含 package）
        assert_eq!(schemes[0].start_cmd, "java -jar svc/target/app.jar");
        // build_cmd = mvn package
        assert_eq!(
            schemes[0].build_cmd.as_deref(),
            Some("mvn -f svc/pom.xml clean package -DskipTests")
        );
    }

    #[test]
    fn maven_scheme_quotes_paths_with_spaces() {
        // 路径含空格：-f 和 jar 路径都加引号
        let schemes = DetectService::build_maven_schemes(
            &Some("purus.jar".to_string()),
            "HanXiInfotech OA Sever/hmsoft-boot-jar",
            None,
            None,
            "hmsoft-boot-jar",
            false,
            None,
        );
        assert_eq!(schemes.len(), 1);
        // build_cmd 的 -f 含空格 → 加引号
        assert!(schemes[0]
            .build_cmd
            .as_ref()
            .unwrap()
            .contains("mvn -f \"HanXiInfotech OA Sever/hmsoft-boot-jar/pom.xml\" clean package"),
            "expected quoted -f in build_cmd");
        // start_cmd 的 jar 路径含空格 → 加引号
        assert!(schemes[0]
            .start_cmd
            .contains("java -jar \"HanXiInfotech OA Sever/hmsoft-boot-jar/target/purus.jar\""),
            "expected quoted jar path in: {}", schemes[0].start_cmd);
    }

    #[test]
    fn maven_scheme_zip_layout_with_release() {
        // ZIP layout + release/：start_cmd = cd release && java -jar；build_cmd = mvn package
        let schemes = DetectService::build_maven_schemes(
            &Some("purus.jar".to_string()),
            "HanXiInfotech OA Sever/hmsoft-boot-jar",
            None,
            Some("HanXiInfotech OA Sever/pom.xml"),
            "hmsoft-boot-jar",
            true, // ZIP layout
            Some("HanXiInfotech OA Sever/hmsoft-boot-jar/release"),
        );
        assert_eq!(schemes.len(), 1);
        // start_cmd 从 release/ 运行
        assert_eq!(
            schemes[0].start_cmd,
            "cd \"HanXiInfotech OA Sever/hmsoft-boot-jar/release\" && java -Dloader.path=./lib -jar purus.jar"
        );
        // build_cmd 走聚合器 -pl -am
        assert!(schemes[0]
            .build_cmd
            .as_ref()
            .unwrap()
            .contains("mvn -f \"HanXiInfotech OA Sever/pom.xml\" -pl hmsoft-boot-jar -am clean package"));
    }

    #[test]
    fn plain_jar_pom_produces_one_scheme() {
        let schemes = DetectService::build_maven_schemes(
            &None,
            "cli",
            None,
            None,
            "cli",
            false,
            None,
        );
        assert_eq!(schemes.len(), 1);
        assert!(schemes[0].recommended);
        // start_cmd 只 java -jar
        assert!(schemes[0].start_cmd.contains("java -jar cli/target/"));
        // build_cmd = mvn package
        assert_eq!(
            schemes[0].build_cmd.as_deref(),
            Some("mvn -f cli/pom.xml clean package -DskipTests")
        );
    }

    #[test]
    fn spring_boot_scheme_when_module_is_root() {
        // 模块即扫描根（mod_rel 空）
        let schemes = DetectService::build_maven_schemes(
            &Some("app.jar".to_string()),
            "",
            None,
            None,
            "repo",
            false,
            None,
        );
        assert_eq!(schemes.len(), 1);
        // start_cmd 用 target/（无子目录前缀）
        assert_eq!(schemes[0].start_cmd, "java -jar target/app.jar");
        // build_cmd 用 -f pom.xml
        assert_eq!(
            schemes[0].build_cmd.as_deref(),
            Some("mvn -f pom.xml clean package -DskipTests")
        );
    }

    #[test]
    fn spring_boot_scheme_injects_java_home() {
        // 有 JAVA_HOME 时：build_cmd 前缀 set JAVA_HOME；start_cmd 用全路径 java
        let schemes = DetectService::build_maven_schemes(
            &Some("app.jar".to_string()),
            "",
            Some("C:\\JAVA\\jdk-21.0.11"),
            None,
            "repo",
            false,
            None,
        );
        // start_cmd 用全路径 java
        assert_eq!(
            schemes[0].start_cmd,
            "\"C:\\JAVA\\jdk-21.0.11\\bin\\java.exe\" -jar target/app.jar"
        );
        // build_cmd 前缀 set JAVA_HOME
        assert!(schemes[0]
            .build_cmd
            .as_ref()
            .unwrap()
            .contains("set \"JAVA_HOME=C:\\JAVA\\jdk-21.0.11\" && mvn -f pom.xml clean package"));
    }

    #[tokio::test]
    async fn scan_extracts_java_home_from_start_bat() {
        // 扫描根有 start.bat 声明 JAVA_HOME，应提取并注入到命令
        // 模拟「审查Agent」场景：root 下 pom.xml + start.bat(含 JAVA_HOME)
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        write(
            &root.join("pom.xml"),
            r#"<project>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.3.4</version></parent>
  <artifactId>review-agent</artifactId>
  <version>1.0.0</version>
</project>"#,
        );
        write_springboot_main(root, "com/review", "ReviewAgentApplication");
        write(
            &root.join("src/main/resources/application.yml"),
            "server:\n  port: 8388\n",
        );
        // start.bat 声明 JAVA_HOME（多 JDK 项目）
        write(
            &root.join("start.bat"),
            "@echo off\nset JAVA_HOME=C:\\JAVA\\jdk-21.0.11\n\"%JAVA_HOME%\\bin\\java.exe\" -jar target\\review-agent-1.0.0.jar\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        // 单方案：分步执行（start_cmd 只 java -jar，build_cmd = mvn package）
        let scheme = &detected[0].schemes[0];
        // start_cmd 只 java -jar：用全路径 java（Java 21），不含 mvn
        assert!(scheme
            .start_cmd
            .contains("\"C:\\JAVA\\jdk-21.0.11\\bin\\java.exe\""));
        assert!(scheme.start_cmd.contains("target/review-agent-1.0.0.jar"));
        assert!(!scheme.start_cmd.contains("mvn"));
        // build_cmd = mvn package（注入 JAVA_HOME 前缀）
        assert!(scheme.build_cmd.is_some());
        assert!(scheme
            .build_cmd
            .as_ref()
            .unwrap()
            .contains("set \"JAVA_HOME=C:\\JAVA\\jdk-21.0.11\" && mvn"));
        assert!(scheme
            .build_cmd
            .as_ref()
            .unwrap()
            .contains("clean package"));
    }

    #[tokio::test]
    async fn scan_springboot_at_scan_root_uses_pom_xml() {
        // 扫描根目录本身就是启动模块（pom.xml 在根）
        // 模拟「审查Agent」场景：root 下直接有 pom.xml + main + application.yml
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        write(
            &root.join("pom.xml"),
            r#"<project>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.3.4</version></parent>
  <artifactId>review-agent</artifactId>
</project>"#,
        );
        write_springboot_main(root, "com/review", "ReviewAgentApplication");
        write(
            &root.join("src/main/resources/application.yml"),
            "server:\n  port: 8388\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name, "review-agent");
        // 模块即扫描根：build_cmd 用 -f pom.xml（不含目录名前缀），start_cmd 用 target/
        let build_cmd = detected[0].schemes[0]
            .build_cmd
            .as_deref()
            .expect("应有 build_cmd");
        assert!(
            build_cmd.contains("-f pom.xml "),
            "expected -f pom.xml in {build_cmd}"
        );
        assert!(!build_cmd.contains("-f review-agent/"));
        // start_cmd 用 target/（无子目录前缀）
        assert!(detected[0].schemes[0]
            .start_cmd
            .contains("java -jar target/"));
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
        assert_eq!(back.schemes.len(), 1);
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
    async fn scan_springboot_port_from_sibling_resource_dir() {
        // 启动模块自身无 src/main/resources，application.yml 放在兄弟模块、
        // 经 pom <resource><directory>../hmsoft-boot/src/main/resources</directory> 引入。
        // 真实「hmsoft-boot-jar」场景：应兜底扫到兄弟模块的端口（8198），而非被判为类库过滤。
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        // 启动模块 hmsoft-boot-jar：有 main，无本地 resources
        write(
            &root.join("hmsoft-boot-jar/pom.xml"),
            r#"<project>
  <artifactId>hmsoft-boot-jar</artifactId>
  <packaging>jar</packaging>
  <parent><groupId>com.hmsoft</groupId>
    <artifactId>hmsoft-purus-bpm</artifactId>
    <version>7.1.0</version></parent>
  <build>
    <finalName>purus</finalName>
    <resources>
      <resource>
        <directory>src/main/resources</directory>
      </resource>
      <resource>
        <directory>../hmsoft-boot/src/main/resources</directory>
        <includes>
          <include>application.yml</include>
        </includes>
      </resource>
    </resources>
  </build>
</project>"#,
        );
        write_springboot_main(
            &root.join("hmsoft-boot-jar"),
            "com/hmsoft",
            "StartBootApplication",
        );
        // 兄弟模块 hmsoft-boot 持有 application.yml（端口 8198）
        write(
            &root.join("hmsoft-boot/src/main/resources/application.yml"),
            "server:\n  port: 8198\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1, "hmsoft-boot-jar 应被检测到");
        assert_eq!(detected[0].name, "hmsoft-boot-jar");
        assert_eq!(detected[0].expected_ports, vec!["8198".to_string()]);
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

    #[tokio::test]
    async fn scan_skips_data_and_repos_dirs() {
        // data/、repos/、logs/ 等数据目录下的克隆仓库不应被扫描
        // 模拟「审查Agent」场景：真实项目在根，data/repos 下有克隆的 pom.xml
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        // 真实后端项目（根 pom）
        write(
            &root.join("pom.xml"),
            r#"<project>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.3.4</version></parent>
  <artifactId>real-app</artifactId>
</project>"#,
        );
        write_springboot_main(root, "com/example", "RealAppApplication");
        write(
            &root.join("src/main/resources/application.yml"),
            "server:\n  port: 8388\n",
        );

        // data/repos 下的克隆仓库（应被跳过，不产生检测结果）
        write(
            &root.join("data/repos/1/pom.xml"),
            r#"<project>
  <artifactId>cloned-aggregator</artifactId>
  <packaging>pom</packaging>
</project>"#,
        );
        write_springboot_main(&root.join("data/repos/1/fake-svc"), "com/x", "FakeSvc");
        write(
            &root.join("data/repos/1/fake-svc/pom.xml"),
            r#"<project><artifactId>fake-svc</artifactId></project>"#,
        );
        write(
            &root.join("data/repos/1/fake-svc/src/main/resources/application.properties"),
            "server.port=9999\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        // 只应扫到根的真实项目，data/repos 下的克隆项目被跳过
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].name, "real-app");
        assert!(detected.iter().all(|d| d.name != "fake-svc"));
    }

    #[tokio::test]
    async fn scan_finds_frontend_after_springboot() {
        // pom 命中后应继续下钻，发现同项目下的 web/ 前端子项目
        // 模拟「审查Agent」场景：根 SpringBoot + web/package.json (Vue)
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        // 根 SpringBoot
        write(
            &root.join("pom.xml"),
            r#"<project>
  <parent><groupId>org.springframework.boot</groupId>
    <artifactId>spring-boot-starter-parent</artifactId>
    <version>3.3.4</version></parent>
  <artifactId>review-agent</artifactId>
</project>"#,
        );
        write_springboot_main(root, "com/review", "ReviewAgentApplication");
        write(
            &root.join("src/main/resources/application.yml"),
            "server:\n  port: 8388\n",
        );

        // web 前端子项目（应被继续下钻扫到）
        write(
            &root.join("web/package.json"),
            r#"{"name":"review-agent-web","scripts":{"dev":"vite"},"devDependencies":{"vite":"^5.0.0"}}"#,
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        // 2 个项目：后端 + 前端
        assert_eq!(detected.len(), 2);
        assert!(detected.iter().any(|d| d.name == "review-agent"));
        assert!(detected.iter().any(|d| d.name == "review-agent-web"));
    }

    #[tokio::test]
    async fn scan_finds_server_and_web_after_root_pkg() {
        // 根 package.json 命中后应继续下钻，发现 server/ 和 web/ 子项目
        // 模拟「AI代理」场景：根 Node + server/ + web/ 子项目
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        // 根 Node 项目（有 dev 脚本，非 workspace）
        write(
            &root.join("package.json"),
            r#"{"name":"proxy-root","scripts":{"dev":"cd server && node src/index.js"}}"#,
        );
        // server 后端
        write(
            &root.join("server/package.json"),
            r#"{"name":"proxy-server","scripts":{"start":"node src/index.js"}}"#,
        );
        // web 前端
        write(
            &root.join("web/package.json"),
            r#"{"name":"proxy-web","scripts":{"dev":"vite"},"devDependencies":{"vite":"^5.0.0"}}"#,
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        // 3 个项目：根 + server + web
        assert_eq!(detected.len(), 3);
        let names: Vec<&str> = detected.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"proxy-root"));
        assert!(names.contains(&"proxy-server"));
        assert!(names.contains(&"proxy-web"));
    }

    #[tokio::test]
    async fn scan_source_port_env_fallback() {
        // 纯后端 Node 项目：端口在源码 process.env.PORT || 3001
        // 模拟「AI测试」server 场景
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("server/package.json"),
            r#"{"name":"hr-server","scripts":{"start":"node src/index.js"},"dependencies":{"express":"^4.0.0"}}"#,
        );
        write(
            &root.join("server/src/index.js"),
            "const PORT = process.env.PORT || 3001;\napp.listen(PORT, () => {});\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].expected_ports, vec!["3001".to_string()]);
    }

    #[tokio::test]
    async fn scan_source_port_listen_direct() {
        // 纯后端 Node 项目：端口在 .listen(3000)
        // 模拟「AI代理」server 场景（Fastify 直接 listen 数字）
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("server/package.json"),
            r#"{"name":"fastify-server","scripts":{"start":"node src/app.js"}}"#,
        );
        write(
            &root.join("server/src/app.js"),
            "const fastify = Fastify();\nfastify.listen(3000, '0.0.0.0');\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].expected_ports, vec!["3000".to_string()]);
    }

    #[tokio::test]
    async fn scan_source_port_env_fallback_quoted() {
        // 端口写法带引号：process.env.PORT || '3000'（config.js 里 parseInt 包裹）
        // 模拟「AI代理」server 真实场景：端口在 src/config.js，index.js 引用 config.port
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        write(
            &root.join("server/package.json"),
            r#"{"name":"proxy-server","scripts":{"start":"node src/index.js"}}"#,
        );
        // config.js 定义端口（带引号字符串）
        write(
            &root.join("server/src/config.js"),
            "export default { port: parseInt(process.env.PORT || '3000') };\n",
        );
        // index.js 引用变量（无法静态推断，靠扫 config.js 兜底）
        write(
            &root.join("server/src/index.js"),
            "import config from './config.js'\nawait fastify.listen({ port: config.port })\n",
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].expected_ports, vec!["3000".to_string()]);
    }

    #[tokio::test]
    async fn scan_coordination_script_root_excluded() {
        // 协调脚本根：多个脚本都以 cd <子目录> && 开头，根不入结果，继续下钻找子项目
        // 模拟「AI代理」真实场景：根 dev/web 都是 cd 转发
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        // 根协调脚本（dev 和 web 都 cd 到子目录）
        write(
            &root.join("package.json"),
            r#"{"name":"claude-deepseek-proxy","scripts":{"dev":"cd server && node --watch src/index.js","web":"cd web && npm run dev","install:all":"cd server && npm install && cd ../web && npm install"}}"#,
        );
        // server 后端
        write(
            &root.join("server/package.json"),
            r#"{"name":"proxy-server","scripts":{"start":"node src/index.js"}}"#,
        );
        write(
            &root.join("server/src/config.js"),
            "export default { port: parseInt(process.env.PORT || '3000') };\n",
        );
        // web 前端
        write(
            &root.join("web/package.json"),
            r#"{"name":"proxy-web","scripts":{"dev":"vite"},"devDependencies":{"vite":"^5.0.0"}}"#,
        );

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        // 2 个项目：server + web（根协调脚本不入结果）
        assert_eq!(detected.len(), 2);
        let names: Vec<&str> = detected.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"proxy-server"));
        assert!(names.contains(&"proxy-web"));
        // 根协调脚本不入结果
        assert!(!names.contains(&"claude-deepseek-proxy"));
        // server 端口应从 config.js 兜底扫到
        let server = detected.iter().find(|d| d.name == "proxy-server").unwrap();
        assert_eq!(server.expected_ports, vec!["3000".to_string()]);
    }
}
