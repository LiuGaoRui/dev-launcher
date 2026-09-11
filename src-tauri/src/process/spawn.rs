//! 进程 spawn：tokio::process + Windows creation_flags + stdio→file
//!
//! 设计要点（见 ADR-001）：
//! - 统一用 `cmd /C <start_cmd>` 执行，支持 `&&`、管道、重定向
//! - stdout/stderr 重定向到日志文件（两独立 append 句柄，OS 负责交错）
//! - `CREATE_NO_WINDOW` 避免弹出控制台窗口
//! - 进程**脱离**管理器生命周期：不设 kill_on_drop、不挂 Job Object，
//!   关闭本软件后子进程继续运行；停止改由 stop_project 的 taskkill /F /T 完成

use crate::error::{AppError, AppResult};
use crate::models::Project;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Windows CREATE_NO_WINDOW 标志，避免子进程弹出控制台窗口。
#[cfg(windows)]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Java 进程的默认字符集参数。
///
/// 必要性：`new String(bytes)` / `String.getBytes()` 用的是 JVM **平台默认字符集**，
/// 命令行启动的 JVM 在中文 Windows 下为 GBK（IDE 启动通常会注入 UTF-8，Linux 生产
/// 环境 locale 也是 UTF-8，故问题只在命令行启动时暴露）。显式指定后行为不再依赖启动方式。
///
/// 探测模板（detect_service）与本模块的 spawn 兜底共用同一常量，避免两处写法漂移。
pub const JAVA_UTF8_OPT: &str = "-Dfile.encoding=UTF-8";

/// 按空白切分命令，但双引号内的空白不切分；返回每个 token 的字节区间。
fn split_tokens(cmd: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut in_quote = false;
    let mut start: Option<usize> = None;
    for (i, c) in cmd.char_indices() {
        if c == '"' {
            in_quote = !in_quote;
            start.get_or_insert(i);
            continue;
        }
        if c.is_whitespace() && !in_quote {
            if let Some(s) = start.take() {
                out.push((s, i));
            }
            continue;
        }
        start.get_or_insert(i);
    }
    if let Some(s) = start {
        out.push((s, cmd.len()));
    }
    out
}

/// 判断 token 是否是 java 可执行文件：`java`、`java.exe`，
/// 或带路径分隔符并以 `\java` / `\java.exe`（`/` 同理）结尾的全路径。
///
/// 刻意不匹配 `-Dfoo=java`、`my-java-app.jar` 这类只是「内容里含 java」的 token。
fn is_java_exe(token: &str) -> bool {
    let t = token.trim_matches('"');
    let lower = t.to_ascii_lowercase();
    if lower == "java" || lower == "java.exe" {
        return true;
    }
    (t.contains('\\') || t.contains('/'))
        && ["\\java", "\\java.exe", "/java", "/java.exe"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

/// 命令分隔符：cmd.exe 下这些位置之后是新命令，其后的 java 才算「命令起始位置」。
fn is_command_separator(token: &str) -> bool {
    matches!(token, "&&" | "&" | "|" | "||")
}

/// 给 Java 启动命令补上 [`JAVA_UTF8_OPT`]；非 Java 命令原样返回。
///
/// 规则：
/// - 只改写处于**命令起始位置**的 java 可执行文件（`java` / `java.exe` /
///   `"…\bin\java.exe"` 全路径），`cd x && java …`、`a && java …` 都能识别
/// - 命令里已出现 `-Dfile.encoding` 时原样返回（不重复注入）
/// - 只在原串 java 可执行文件之后插入，其余字符（引号、`&&` 前后空格）一律不动
pub fn inject_java_utf8(start_cmd: &str) -> String {
    if start_cmd.contains("-Dfile.encoding") {
        return start_cmd.to_string();
    }
    let tokens = split_tokens(start_cmd);
    for (i, &(s, e)) in tokens.iter().enumerate() {
        if !is_java_exe(&start_cmd[s..e]) {
            continue;
        }
        let at_command_start = i == 0 || {
            let (ps, pe) = tokens[i - 1];
            is_command_separator(&start_cmd[ps..pe])
        };
        if !at_command_start {
            continue;
        }
        let mut out = String::with_capacity(start_cmd.len() + JAVA_UTF8_OPT.len() + 1);
        out.push_str(&start_cmd[..e]);
        out.push(' ');
        out.push_str(JAVA_UTF8_OPT);
        out.push_str(&start_cmd[e..]);
        return out;
    }
    start_cmd.to_string()
}

/// 计算项目启动日志文件路径：`{logs_root}/{project_id}/start.log`
///
/// 同时确保目录存在。返回日志文件路径。
/// 启动日志固定单文件（start.log），由 start_project 在 spawn 前 truncate，
/// 故只含「本次」启动输出；tail 实时订阅可完整看到。
pub fn start_log_path(logs_root: &Path, project_id: i64) -> AppResult<PathBuf> {
    let path = crate::logs::paths::log_path_of(logs_root, project_id, crate::logs::paths::LogType::Start);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}

/// 往日志文件追加一行（create + append，best-effort）。
///
/// 用于「执行前已 truncate，但 spawn 失败」的场景：把错误原因写进日志文件，
/// 避免用户打开日志看到空白、误以为没报错。
pub fn append_log_line(path: &Path, line: &str) {
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

/// Truncate 一个日志文件：文件存在则清空内容（保留文件本身），不存在视为 no-op。
///
/// 供 start_project（start.log）与 build_project（build.log）在执行前调用，
/// 保证每次执行只保留本次输出。文件不存在时不创建——tail 的 OPEN_RETRY 会处理
/// 「文件尚未创建」的等待，写日志时也会自动创建。
pub fn truncate_log(path: &Path) {
    if path.exists() {
        // 已存在才 truncate，避免在尚未写日志的项目目录下留下空文件
        if let Ok(file) = OpenOptions::new().write(true).truncate(true).open(path) {
            drop(file);
        }
    }
}

/// 打开日志文件的两个 append 句柄（stdout / stderr 各一），OS 负责交错写入。
///
/// Stdio::from 会 move File，故用 try_clone 复制句柄而非重新打开。
pub(crate) fn open_log_stdio(log_path: &Path) -> AppResult<(Stdio, Stdio)> {
    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let stderr_file = stdout_file.try_clone()?;
    Ok((Stdio::from(stdout_file), Stdio::from(stderr_file)))
}

/// 计算项目命令的执行目录（cwd），供 start_cmd / build_cmd 共用。
///
/// 必须与 `detect_service.rs` 生成命令时假定的基准目录一致：
/// - Java 类（Springboot/JavaJar）：命令里的相对路径（jar 路径、mvn -f、release/）
///   均以 `workdir`（= scan_root）为基准生成，故 cwd 用 `workdir`（无则回退 `path`）。
///   典型：多模块项目 workdir 在 reactor 根，-f 写成 `子模块/pom.xml`。
/// - 其他（Node/DockerCompose/Custom）：命令以 `path`（package.json 所在）为基准。
///
/// **start_cmd 与 build_cmd 必须用同一套 cwd 逻辑**，否则相对路径会解析错位
/// （如 build_cmd 的 `-f 子模块/pom.xml` 在模块目录下找不到）。
pub fn resolve_cwd(project: &Project) -> &str {
    match project.r#type {
        crate::models::ProjectType::Springboot | crate::models::ProjectType::JavaJar => {
            project.workdir.as_deref().unwrap_or(&project.path)
        }
        _ => &project.path,
    }
}

/// spawn 一个项目进程。
///
/// - 用 `cmd /C <start_cmd>` 执行
/// - current_dir 用 [`resolve_cwd`]（按项目类型区分）
/// - stdout/stderr 重定向到 `log_path`
/// - 返回 (Child, pid)；pid 来自 `child.id()`
pub fn spawn_command(project: &Project, log_path: &Path) -> AppResult<(Child, u32)> {
    // Java 项目兜底补上 UTF-8 默认字符集：存量项目的 start_cmd 里没有该参数，
    // 靠这里统一注入，不必逐个项目重新探测（详见 inject_java_utf8 的说明）。
    let effective_cmd = inject_java_utf8(&project.start_cmd);
    if effective_cmd != project.start_cmd {
        // 记一笔实际执行命令，便于排查「UI 里保存的命令」与「实际执行」的差异；写失败不影响启动
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path) {
            let _ = writeln!(
                f,
                "# [项目管理器] 已自动追加 {JAVA_UTF8_OPT}，实际执行命令：{effective_cmd}"
            );
        }
    }

    let (stdout, stderr) = open_log_stdio(log_path)?;

    let mut cmd = Command::new("cmd");
    // raw_arg 不做转义，整串交给 cmd.exe 解析（支持 &&、管道、重定向）
    cmd.raw_arg(format!("/C {}", effective_cmd));
    cmd.current_dir(resolve_cwd(project));
    cmd.stdin(Stdio::null());
    cmd.stdout(stdout);
    cmd.stderr(stderr);
    // 不设 kill_on_drop：进程脱离管理器生命周期，关闭软件后子进程继续运行。

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let child = cmd
        .spawn()
        .map_err(|e| AppError::Process(format!("启动项目「{}」失败: {e}", project.name)))?;

    let pid = child
        .id()
        .ok_or_else(|| AppError::Process(format!("启动「{}」后无法获取 PID", project.name)))?;

    Ok((child, pid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_log_path_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let file = start_log_path(tmp.path(), 7).unwrap();
        let dir = file.parent().unwrap();
        assert!(dir.exists());
        assert!(file.to_string_lossy().ends_with("start.log"));
        assert_eq!(dir.file_name().unwrap(), "7");
    }

    #[test]
    fn inject_utf8_bare_java() {
        assert_eq!(
            inject_java_utf8("java -jar target/app.jar"),
            "java -Dfile.encoding=UTF-8 -jar target/app.jar"
        );
    }

    #[test]
    fn inject_utf8_after_cd_prefix_keeps_loader_path() {
        assert_eq!(
            inject_java_utf8(
                "cd \"HanXiInfotech OA Sever/hmsoft-boot-jar/release\" && java -Dloader.path=./lib -jar purus.jar"
            ),
            "cd \"HanXiInfotech OA Sever/hmsoft-boot-jar/release\" && java -Dfile.encoding=UTF-8 -Dloader.path=./lib -jar purus.jar"
        );
    }

    #[test]
    fn inject_utf8_quoted_full_path_java_exe() {
        assert_eq!(
            inject_java_utf8("\"C:\\JAVA\\jdk-21.0.11\\bin\\java.exe\" -jar target/app.jar"),
            "\"C:\\JAVA\\jdk-21.0.11\\bin\\java.exe\" -Dfile.encoding=UTF-8 -jar target/app.jar"
        );
    }

    #[test]
    fn inject_utf8_does_not_duplicate_existing_flag() {
        let cmd = "java -Dfile.encoding=UTF-8 -jar target/app.jar";
        assert_eq!(inject_java_utf8(cmd), cmd);
    }

    #[test]
    fn inject_utf8_leaves_non_java_commands_untouched() {
        for cmd in [
            "npm run dev",
            "pnpm dev",
            "docker compose up -d",
            "node server.js",
            "./gradlew bootRun",
        ] {
            assert_eq!(inject_java_utf8(cmd), cmd, "不应改写非 Java 命令: {cmd}");
        }
    }

    #[test]
    fn inject_utf8_skips_java_used_as_argument() {
        // 只有命令起始位置的 java 才是可执行文件；`echo java` 里的 java 是参数
        assert_eq!(inject_java_utf8("echo java"), "echo java");
    }

    #[test]
    fn inject_utf8_ignores_jar_name_containing_java() {
        assert_eq!(
            inject_java_utf8("java -jar target/my-java-app.jar"),
            "java -Dfile.encoding=UTF-8 -jar target/my-java-app.jar"
        );
    }
}
