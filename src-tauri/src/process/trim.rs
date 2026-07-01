//! 进程内存回收：修剪工作集（Trim Working Set）。
//!
//! 与 kill.rs 的杀进程不同，trim 不终止进程，而是请求 OS 把进程不活跃的
//! 物理内存页换出到 pagefile，立即降低 RSS（驻留集大小）。进程继续运行，
//! 下次访问被换出的页时由 OS 按需换入（轻微缺页代价，远低于重启）。
//!
//! Windows 实现：`SetProcessWorkingSetSizeEx(handle, -1, -1, 0)`。
//! MSDN：当 dwMinimum/MaximumWorkingSetSize 均为 `(SIZE_T)-1` 时，函数会从
//! 该进程工作集中尽可能多地移除页面。这正是任务管理器「降低内存占用」、
//! 以及各种内存整理工具背后的同一机制。
//!
//! 适用场景：IDE（IDEA=JVM、VSCode=Electron）的堆中大量不活跃页，
//! 回收后立即腾出物理内存而不打断用户工作。

use crate::error::{AppError, AppResult};

/// 修剪单个进程的工作集，请求 OS 移除尽可能多的物理页。
///
/// 只负责 OS 调用本身；释放量统计（回收前后 RSS 差值）在 cleaner 命令层完成，
/// 那里能统一持锁刷新 sysinfo。
///
/// 返回 `Ok(())` 表示调用成功（实际释放量由 OS 决定，可能为 0）。
#[cfg(windows)]
pub fn trim_working_set(pid: u32) -> AppResult<()> {
    use windows::Win32::System::Memory::{SetProcessWorkingSetSizeEx, SETPROCESSWORKINGSETSIZEEX_FLAGS};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
    };
    use windows::core::Owned;

    // SAFETY: OpenProcess 按 PID 打开进程句柄。请求 SET_QUOTA（修剪工作集所需）
    // + QUERY_LIMITED_INFORMATION（读取信息所需）。非自身进程需要足够权限
    // （与 DevLauncher 同用户启动的 IDE 进程通常可访问）。
    let raw_handle = unsafe {
        OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
    }
    .map_err(|e| AppError::Process(format!("OpenProcess(pid={pid}) 失败: {e}")))?;
    let handle = unsafe { Owned::new(raw_handle) };

    // SAFETY: handle 由 OpenProcess 校验有效。传 -1/-1 是 MSDN 文档约定的
    // 「尽可能修剪」信号。dwFlags 为空表示不改变配额限制，仅执行一次性修剪。
    let result = unsafe {
        SetProcessWorkingSetSizeEx(
            *handle,
            usize::MAX, // dwMinimumWorkingSetSize = (SIZE_T)-1
            usize::MAX, // dwMaximumWorkingSetSize = (SIZE_T)-1
            SETPROCESSWORKINGSETSIZEEX_FLAGS::default(),
        )
    };

    match result {
        Ok(()) => Ok(()),
        // 常见错误：进程已退出(ERROR_INVALID_PARAMETER)、权限不足(ERROR_ACCESS_DENIED)
        Err(e) => Err(AppError::Process(format!(
            "SetProcessWorkingSetSizeEx(pid={pid}) 失败: {e}"
        ))),
    }
}

/// 非 Windows 平台的占位实现。
#[cfg(not(windows))]
pub fn trim_working_set(pid: u32) -> AppResult<()> {
    let _ = pid;
    Err(AppError::Process(
        "内存回收（修剪工作集）仅支持 Windows".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_nonexistent_pid_returns_error() {
        // 一个几乎不可能存在的 PID；应返回错误而非 panic
        let result = trim_working_set(0xFFFF_FFF0);
        assert!(result.is_err());
    }

    #[cfg(windows)]
    #[test]
    fn trim_self_succeeds() {
        // 修剪自身进程应该成功（权限充足）
        let pid = std::process::id();
        let result = trim_working_set(pid);
        assert!(result.is_ok(), "修剪自身进程应成功: {:?}", result.err());
    }
}
