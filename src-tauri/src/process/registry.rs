//! ProcessRegistry：运行中进程的注册表
//!
//! `Mutex<HashMap<i64, RunningProcess>>`，key 为 project_id。
//! 提供 insert/remove/contains/snapshot 等操作，锁粒度尽量小。
//! RunningProcess 持有 tokio Child + JobHandle，drop 时自动回收（kill_on_drop + KILL_ON_JOB_CLOSE）。

use crate::error::{AppError, AppResult};
use crate::process::job_object::JobHandle;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::process::Child;

/// 一个运行中的项目进程。
pub struct RunningProcess {
    pub child: Child,
    pub job: JobHandle,
    pub pid: u32,
    pub log_path: PathBuf,
    pub started_at: String,
}

/// RunningProcess 的只读快照（避免持锁访问 child）。
#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub log_path: PathBuf,
    pub started_at: String,
}

impl From<&RunningProcess> for ProcessSnapshot {
    fn from(p: &RunningProcess) -> Self {
        Self {
            pid: p.pid,
            log_path: p.log_path.clone(),
            started_at: p.started_at.clone(),
        }
    }
}

/// 全局进程注册表。
pub struct ProcessRegistry {
    inner: Mutex<HashMap<i64, RunningProcess>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// 插入一个运行中进程。若 project_id 已存在返回 `AlreadyRunning`。
    pub fn insert(&self, project_id: i64, proc: RunningProcess) -> AppResult<()> {
        let mut map = self.inner.lock().expect("registry mutex poisoned");
        if map.contains_key(&project_id) {
            return Err(AppError::AlreadyRunning(project_id));
        }
        map.insert(project_id, proc);
        Ok(())
    }

    /// 移除并返回一个进程（用于显式 stop / 退出清理）。幂等。
    pub fn remove(&self, project_id: i64) -> Option<RunningProcess> {
        let mut map = self.inner.lock().expect("registry mutex poisoned");
        map.remove(&project_id)
    }

    /// 是否存在（用于 start 前校验）。
    pub fn contains(&self, project_id: i64) -> bool {
        let map = self.inner.lock().expect("registry mutex poisoned");
        map.contains_key(&project_id)
    }

    /// 取只读快照（不持锁访问 child）。
    pub fn snapshot(&self, project_id: i64) -> Option<ProcessSnapshot> {
        let map = self.inner.lock().expect("registry mutex poisoned");
        map.get(&project_id).map(ProcessSnapshot::from)
    }

    /// 列出所有运行中 project_id（供 monitor 批量查询）。
    pub fn running_ids(&self) -> Vec<i64> {
        let map = self.inner.lock().expect("registry mutex poisoned");
        map.keys().copied().collect()
    }

    /// 在持锁回调中操作指定进程（用于 stop 时调 job.terminate）。
    /// 回调返回值的 Result 透传。
    pub fn with_mut<R>(
        &self,
        project_id: i64,
        f: impl FnOnce(&mut RunningProcess) -> R,
    ) -> Option<R> {
        let mut map = self.inner.lock().expect("registry mutex poisoned");
        map.get_mut(&project_id).map(f)
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    /// 构造一个最小化的 RunningProcess（不含真实 Child/Job，用 mock）。
    ///
    /// 注意：JobHandle::new() 会真实创建 Job Object，但单元测试无需 spawn 真进程。
    fn make_proc(pid: u32) -> RunningProcess {
        // 用一个立即退出的 dummy 进程占位 child（ping 之类不实际启动，仅占位结构）。
        // 这里用一个会立即结束的命令，确保 child 字段合法。
        let mut cmd = tokio::process::Command::new("cmd");
        cmd.arg("/C").arg("exit 0");
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        let child = cmd.spawn().expect("spawn dummy child");
        RunningProcess {
            child,
            job: JobHandle::new().expect("create job"),
            pid,
            log_path: PathBuf::from("/tmp/test.log"),
            started_at: "2026-01-01 00:00:00".into(),
        }
    }

    #[test]
    fn insert_get_remove() {
        let reg = ProcessRegistry::new();
        assert!(!reg.contains(1));

        reg.insert(1, make_proc(100)).unwrap();
        assert!(reg.contains(1));

        let snap = reg.snapshot(1).unwrap();
        assert_eq!(snap.pid, 100);

        assert!(reg.remove(1).is_some());
        assert!(!reg.contains(1));
        assert!(reg.snapshot(1).is_none());
    }

    #[test]
    fn duplicate_insert_errors() {
        let reg = ProcessRegistry::new();
        reg.insert(1, make_proc(100)).unwrap();
        let err = reg.insert(1, make_proc(200)).unwrap_err();
        assert!(matches!(err, AppError::AlreadyRunning(1)));
    }

    #[test]
    fn remove_nonexistent_is_none() {
        let reg = ProcessRegistry::new();
        assert!(reg.remove(999).is_none());
    }

    #[test]
    fn running_ids_and_with_mut() {
        let reg = ProcessRegistry::new();
        reg.insert(1, make_proc(100)).unwrap();
        reg.insert(2, make_proc(200)).unwrap();

        let mut ids = reg.running_ids();
        ids.sort();
        assert_eq!(ids, vec![1, 2]);

        let pid_via_with = reg.with_mut(1, |p| p.pid).unwrap();
        assert_eq!(pid_via_with, 100);
        assert!(reg.with_mut(999, |_| ()).is_none());
    }
}
