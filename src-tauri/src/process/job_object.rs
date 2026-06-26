//! Windows Job Object 封装（unsafe FFI）
//!
//! 设计要点（见 ADR-001 / 调研结论）：
//! - 用 `Owned<HANDLE>` 做 RAII：drop 时自动 `CloseHandle`，配合
//!   `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 自动杀掉 Job 内全部进程（含后代）
//! - 杀树可靠性依赖 Job Object，而非 `taskkill /T`
//! - MVP 仅 Windows；非 Windows 平台用 `cfg` 占位
//!
//! HANDLE 内部是裸指针（非 Send/Sync），但 Job 句柄在单线程内使用、
//! 通过 Mutex 串行访问，故 `JobHandle` 显式 unsafe impl Send/Sync。

use crate::error::{AppError, AppResult};
use std::mem::size_of;

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::os::windows::io::RawHandle;
    use windows::core::{Owned, PCWSTR};
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, QueryInformationJobObject,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicAccountingInformation,
        JobObjectExtendedLimitInformation, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    /// Job Object 的 RAII 句柄。
    ///
    /// drop 时关闭句柄，因设置了 `KILL_ON_JOB_CLOSE`，OS 自动终止 Job 内所有进程。
    pub struct JobHandle {
        // Owned<HANDLE> 非 Send/Sync（含裸指针），但本结构经 Mutex 串行访问，安全。
        handle: Owned<HANDLE>,
    }

    // SAFETY: JobHandle 仅在 ProcessRegistry 的 Mutex 内访问，串行化使用。
    // HANDLE 是 OS 内核对象的弱引用句柄，跨线程传递本身是安全的（OS 保证）。
    unsafe impl Send for JobHandle {}
    unsafe impl Sync for JobHandle {}

    impl JobHandle {
        /// 创建匿名 Job 并设置 kill-on-close。
        pub fn new() -> AppResult<Self> {
            // SAFETY: CreateJobObjectW 传 None（默认安全属性）+ null 名称（匿名 Job），
            // 返回的 HANDLE 由 Owned 包裹管理生命周期。
            let handle = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
                .map_err(|e| AppError::Process(format!("CreateJobObjectW 失败: {e}")))?;

            // 设置 JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE：Job 句柄关闭时杀掉全部子进程
            // SAFETY: handle 有效，info 结构体零初始化后只改 LimitFlags 字段，长度匹配。
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            unsafe {
                SetInformationJobObject(
                    handle.clone(),
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const std::ffi::c_void,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            }
            .map_err(|e| AppError::Process(format!("SetInformationJobObject 失败: {e}")))?;

            Ok(Self {
                // SAFETY: CreateJobObjectW 返回 Ok 即句柄有效（已做 win32 错误检查），
                // 用 Owned 包裹接管所有权，drop 时自动 CloseHandle。
                handle: unsafe { Owned::new(handle) },
            })
        }

        /// 把一个进程（按 raw handle）加入 Job。
        ///
        /// 调用方需保证传入的是刚 spawn 的进程句柄（tokio Child::raw_handle）。
        /// 存在微秒级 spawn-then-assign 竞态，由 KILL_ON_JOB_CLOSE 兜底（见 ADR-001 注记）。
        pub fn assign(&self, process: RawHandle) -> AppResult<()> {
            // SAFETY: handle 有效；process 是 std/tokio 提供的有效进程句柄。
            let h = HANDLE(process);
            let job = self.handle.clone();
            unsafe { AssignProcessToJobObject(job, h) }
                .map_err(|e| AppError::Process(format!("AssignProcessToJobObject 失败: {e}")))
        }

        /// 终止 Job 内全部进程（整树）。
        pub fn terminate(&self) -> AppResult<()> {
            // SAFETY: handle 有效，退出码 1 为通用值。
            let job = self.handle.clone();
            unsafe { TerminateJobObject(job, 1) }
                .map_err(|e| AppError::Process(format!("TerminateJobObject 失败: {e}")))
        }

        /// 查询 Job 当前活跃进程数（用于 stop 时轮询确认进程树已退出）。
        pub fn active_processes(&self) -> AppResult<u32> {
            let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
            let job = self.handle.clone();
            // SAFETY: handle 有效，info 缓冲区大小与请求的 info class 匹配。
            unsafe {
                QueryInformationJobObject(
                    job,
                    JobObjectBasicAccountingInformation,
                    &mut info as *mut _ as *mut std::ffi::c_void,
                    size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                    None,
                )
            }
            .map_err(|e| AppError::Process(format!("QueryInformationJobObject 失败: {e}")))?;
            Ok(info.ActiveProcesses)
        }
    }
}

#[cfg(windows)]
pub use windows_impl::JobHandle;

#[cfg(all(test, windows))]
mod integration_tests {
    //! 进程 spawn/stop 的集成测试，默认 ignored（涉及真实系统进程）。
    //! 手动跑：`cargo test -- --ignored process::job_object::integration`

    use super::*;
    use crate::process::spawn::spawn_command;
    use crate::models::{Project, ProjectType};
    use std::time::Duration;

    fn sample_project(cmd: &str) -> Project {
        Project {
            id: 1,
            name: "integration-test".into(),
            group_id: None,
            r#type: ProjectType::Custom,
            path: ".".into(),
            workdir: None,
            start_cmd: cmd.into(),
            build_cmd: None,
            expected_ports: vec![],
            enabled: true,
            last_pid: None,
            last_start_time: None,
            last_stop_time: None,
            create_time: String::new(),
            update_time: String::new(),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn job_terminate_kills_spawned_process() {
        // ping 持续运行，便于观察 kill 效果
        let project = sample_project("ping -n 30 127.0.0.1");
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("test.log");

        let (mut child, _pid) = spawn_command(&project, &log_path).unwrap();
        let raw_handle = child.raw_handle().expect("raw handle");

        let job = JobHandle::new().unwrap();
        job.assign(raw_handle).unwrap();

        // 确认进程在跑、Job 有活跃进程
        assert!(job.active_processes().unwrap() >= 1);

        // 终止
        job.terminate().unwrap();

        // 轮询 ActiveProcesses 归零（最多 5s）
        let mut waited = 0u64;
        loop {
            let active = job.active_processes().unwrap();
            if active == 0 {
                break;
            }
            assert!(waited < 5000, "进程未在 5s 内退出");
            tokio::time::sleep(Duration::from_millis(100)).await;
            waited += 100;
        }

        // child 也应已退出（try_wait 可能需短暂延迟才感知到 OS 已回收）
        let mut w2 = 0u64;
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            assert!(w2 < 2000, "child 未在 2s 内感知到退出");
            tokio::time::sleep(Duration::from_millis(50)).await;
            w2 += 50;
        }
    }

    #[tokio::test]
    #[ignore]
    async fn job_active_processes_empty_after_child_exits() {
        // 立即退出的命令
        let project = sample_project("echo hello");
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("test.log");

        let (child, _pid) = spawn_command(&project, &log_path).unwrap();
        let raw_handle = child.raw_handle().expect("raw handle");

        let job = JobHandle::new().unwrap();
        job.assign(raw_handle).unwrap();

        // 等 echo 退出（最多 2s）
        let mut waited = 0u64;
        loop {
            let active = job.active_processes().unwrap();
            if active == 0 {
                break;
            }
            assert!(waited < 2000, "echo 未在 2s 内退出");
            tokio::time::sleep(Duration::from_millis(100)).await;
            waited += 100;
        }
    }
}

#[cfg(not(windows))]
mod stub_impl {
    use super::*;

    pub struct JobHandle;

    impl JobHandle {
        pub fn new() -> AppResult<Self> {
            Err(AppError::Process("Job Object 仅支持 Windows".into()))
        }
        pub fn assign(&self, _process: std::os::fd::RawFd) -> AppResult<()> {
            unreachable!()
        }
        pub fn terminate(&self) -> AppResult<()> {
            unreachable!()
        }
        pub fn active_processes(&self) -> AppResult<u32> {
            unreachable!()
        }
    }
}

#[cfg(not(windows))]
pub use stub_impl::JobHandle;
