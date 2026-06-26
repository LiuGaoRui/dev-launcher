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
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};

/// Windows CREATE_NO_WINDOW 标志，避免子进程弹出控制台窗口。
#[cfg(windows)]
pub(crate) const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 清洗项目名为合法目录名：非 [A-Za-z0-9_\u4e00-\u9fa5-] 字符替换为 `_`。
///
/// 保留中文字符（\u4e00-\u9fa5 CJK 统一表意文字基本区），Windows 文件系统支持。
///
/// 跨模块复用：spawn（写日志）与 logs/paths（读/列日志）须用同一规则拼目录名，
/// 否则读写路径会错位。故提升为 pub。
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric()
                || c == '_'
                || c == '-'
                || ('\u{4e00}'..='\u{9fa5}').contains(&c)
            {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 计算项目当日日志文件路径：`{logs_root}/{sanitized_name}/{YYYYMMDD}.log`
///
/// 同时确保目录存在。返回日志文件路径。
pub fn log_file_path(logs_root: &Path, project_name: &str) -> AppResult<PathBuf> {
    let dir = logs_root.join(sanitize_name(project_name));
    std::fs::create_dir_all(&dir)?;
    let date = chrono::Local::now().format("%Y%m%d").to_string();
    let file = dir.join(format!("{date}.log"));
    Ok(file)
}

/// 打开日志文件的两个 append 句柄（stdout / stderr 各一），OS 负责交错写入。
///
/// Stdio::from 会 move File，故用 try_clone 复制句柄而非重新打开。
fn open_log_stdio(log_path: &Path) -> AppResult<(Stdio, Stdio)> {
    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let stderr_file = stdout_file.try_clone()?;
    Ok((Stdio::from(stdout_file), Stdio::from(stderr_file)))
}

/// spawn 一个项目进程。
///
/// - 用 `cmd /C <start_cmd>` 执行
/// - current_dir 优先用 `project.workdir`（运行时工作目录），为空则用 `project.path`
/// - stdout/stderr 重定向到 `log_path`
/// - 返回 (Child, pid)；pid 来自 `child.id()`
pub fn spawn_command(project: &Project, log_path: &Path) -> AppResult<(Child, u32)> {
    let (stdout, stderr) = open_log_stdio(log_path)?;

    let mut cmd = Command::new("cmd");
    // raw_arg 不做转义，整串交给 cmd.exe 解析（支持 &&、管道、重定向）
    cmd.raw_arg(format!("/C {}", project.start_cmd));
    // 命令执行目录按项目类型区分：
    // - Java 类（Springboot/JavaJar）：workdir 优先（License 等运行时资源在扫描根），
    //   为空回退 path；mvn 用 -f 参数定位启动模块，故 cwd 在扫描根也能找到 main 类
    // - 其他（Node/DockerCompose/Custom）：用 path（package.json 等在此）
    let cwd = match project.r#type {
        crate::models::ProjectType::Springboot | crate::models::ProjectType::JavaJar => {
            project.workdir.as_deref().unwrap_or(&project.path)
        }
        _ => &project.path,
    };
    cmd.current_dir(cwd);
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
    fn sanitize_replaces_illegal_chars() {
        assert_eq!(sanitize_name("HR 系统后端"), "HR_系统后端");
        assert_eq!(sanitize_name("a/b:c*d"), "a_b_c_d");
        assert_eq!(sanitize_name("normal-name_1"), "normal-name_1");
        assert_eq!(sanitize_name("中文项目"), "中文项目");
    }

    #[test]
    fn log_path_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let file = log_file_path(tmp.path(), "HR 系统/后端").unwrap();
        let dir = file.parent().unwrap();
        assert!(dir.exists());
        assert!(file
            .to_string_lossy()
            .ends_with(&format!("{}.log", chrono::Local::now().format("%Y%m%d"))));
        assert!(dir.to_string_lossy().contains("HR_系统_后端"));
    }
}
