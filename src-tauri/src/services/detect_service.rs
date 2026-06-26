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

/// `detect_maven` 的探测结果：聚合器容器（继续下钻）或真实项目。
enum MavenProbe {
    Aggregator,
    Project(DetectedProject),
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
            // pom.xml：Maven 项目。聚合器（packaging=pom）视为 monorepo 容器，继续下钻
            match Self::detect_maven(root, dir)? {
                Some(MavenProbe::Project(d)) => {
                    out.push(d);
                    return Ok(());
                }
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

        // 推断端口
        let ports = Self::infer_node_ports(pkg);

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

    /// 根据依赖推断 Node 项目端口。
    fn infer_node_ports(pkg: &serde_json::Value) -> Vec<String> {
        // vite 常出现在 devDependencies，故同时查 dependencies + devDependencies
        let in_deps = |section: &str, name: &str| {
            pkg.get(section).and_then(|v| v.get(name)).is_some()
        };
        let has = |n: &str| in_deps("dependencies", n) || in_deps("devDependencies", n);

        if has("vite") {
            vec!["5173".to_string()]
        } else if has("@vue/cli-service") {
            vec!["8080".to_string()]
        } else if has("@angular/cli") {
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
    /// - `Ok(Some(Project(d)))` — 有效项目
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

        // spring-boot 检测用原始内容：继承关系常在 <parent> 里（已被 strip 删除）
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

        // 端口：读 application 配置
        let port = Self::read_java_port(dir);
        let expected_ports = match port {
            Some(p) => vec![p],
            None => Vec::new(),
        };

        let schemes = Self::build_maven_schemes(is_spring_boot, &jar_name);

        Ok(Some(MavenProbe::Project(DetectedProject {
            rel_path: relpath(root, dir),
            path: dir.to_string_lossy().to_string(),
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

    /// 读 Java 应用端口（application.properties / application.yml）。
    fn read_java_port(dir: &Path) -> Option<String> {
        let res = dir.join("src/main/resources");
        // properties
        let props = res.join("application.properties");
        if let Ok(content) = std::fs::read_to_string(&props) {
            if let Some(p) = parse_properties_port(&content) {
                return Some(p);
            }
        }
        // yml（尝试 application.yml / application.yaml）
        for fname in &["application.yml", "application.yaml"] {
            let p = res.join(fname);
            if let Ok(content) = std::fs::read_to_string(&p) {
                if let Some(port) = parse_yml_port(&content) {
                    return Some(port);
                }
            }
        }
        None
    }

    /// 生成 Maven 项目的启动方案。
    fn build_maven_schemes(is_spring_boot: bool, jar_name: &Option<String>) -> Vec<LaunchScheme> {
        // 共用：打包运行命令 / 构建命令（spring-boot 与 plain 仅描述与推荐位不同）
        let jar_start = match jar_name {
            Some(j) => format!("java -jar target/{j}"),
            None => "java -jar target/app.jar".to_string(),
        };
        let package_build = Some("mvn clean package -DskipTests".to_string());

        if is_spring_boot {
            let desc = match jar_name {
                Some(_) => "先构建 jar 再运行，模拟生产形态",
                None => "先构建 jar 再运行；jar 名未能自动识别，请按实际产物修正",
            };
            vec![
                LaunchScheme {
                    label: "开发模式".to_string(),
                    recommended: true,
                    start_cmd: "mvn spring-boot:run".to_string(),
                    build_cmd: None,
                    description: "Maven 直接运行，改代码重跑即可，日常开发最快".to_string(),
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

/// 读取并解析 package.json；读失败或非法 JSON 返回 None。
fn read_pkg_json(path: &Path) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
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

/// 从已预处理（去注释 + 去 parent）的 pom 中取首个 <artifactId>。
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

/// 解析 yml 中 server.port（简易缩进匹配，不引入 yaml 库）。
fn parse_yml_port(content: &str) -> Option<String> {
    let mut in_server = false;
    let mut server_indent: usize = 0;
    for line in content.lines() {
        // 跳过注释
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        // 顶层 server: 块开始
        if indent == 0 && trimmed == "server:" {
            in_server = true;
            server_indent = 0;
            continue;
        }
        if in_server {
            // 更顶层的新块结束 server 段
            if indent <= server_indent && trimmed.ends_with(':') {
                in_server = false;
                continue;
            }
            if let Some((k, v)) = trimmed.split_once(':') {
                if k.trim() == "port" {
                    let v = v.trim().trim_matches('"');
                    if !v.is_empty() {
                        return Some(v.to_string());
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
    fn spring_boot_pom_produces_two_schemes() {
        let schemes = DetectService::build_maven_schemes(true, &Some("app.jar".to_string()));
        assert_eq!(schemes.len(), 2);
        assert!(schemes[0].recommended);
        assert_eq!(schemes[0].start_cmd, "mvn spring-boot:run");
        assert_eq!(schemes[1].start_cmd, "java -jar target/app.jar");
        assert_eq!(
            schemes[1].build_cmd.as_deref(),
            Some("mvn clean package -DskipTests")
        );
    }

    #[test]
    fn plain_jar_pom_produces_one_scheme() {
        let schemes = DetectService::build_maven_schemes(false, &None);
        assert_eq!(schemes.len(), 1);
        assert!(schemes[0].recommended);
        assert!(schemes[0].start_cmd.starts_with("java -jar target/"));
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

        let detected = DetectService::scan(&root.to_string_lossy()).await.unwrap();
        // 2 个项目
        assert_eq!(detected.len(), 2);
        // 按 rel_path 排序：backend/hr < frontend
        let back = &detected[0];
        let front = &detected[1];
        assert_eq!(back.r#type, ProjectType::Springboot);
        assert_eq!(back.name, "hr");
        assert_eq!(back.expected_ports, vec!["8088".to_string()]);
        assert_eq!(back.schemes.len(), 2);
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
}
