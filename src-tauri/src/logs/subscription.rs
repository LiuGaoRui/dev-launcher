//! 日志订阅注册表：显式管理 tail task 生命周期
//!
//! 取代 ADR-002 的「靠前端 GC Channel 自动退订」：GC 时机不确定，且前端
//! 切换项目/类型时旧 task 仍会继续推送，导致日志串入新缓冲。现在每次订阅
//! 由后端分配唯一 id 并登记，前端显式 `unsubscribe_log(sub_id)` 即时取消。
//!
//! 取消用 `AtomicBool` 标志而非 `JoinHandle::abort`：tail task 只需在轮询
//! 间隔处检查标志后干净退出，避免在 `Channel::send` 中途被强杀；task 自然
//! 退出（文件长期不存在 / Channel 关闭）时调 [`LogSubRegistry::finish`] 自清理。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 日志订阅注册表（线程安全，可被 tail task 共享自清理）。
pub struct LogSubRegistry {
    inner: Mutex<HashMap<u64, Arc<AtomicBool>>>,
    next_id: AtomicU64,
}

impl LogSubRegistry {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
        }
    }

    /// 登记一个新订阅，返回 `(订阅 id, 取消标志)`。
    ///
    /// 必须先登记再 spawn task：否则 task 可能先自然退出并调 `finish`，
    /// 随后本次 insert 会留下一个永不清理的残留项。
    pub fn register(&self) -> (u64, Arc<AtomicBool>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let cancel = Arc::new(AtomicBool::new(false));
        self.inner
            .lock()
            .expect("log subs mutex poisoned")
            .insert(id, cancel.clone());
        (id, cancel)
    }

    /// 取消订阅：置取消标志并移除登记。返回该订阅是否存在。
    pub fn cancel(&self, id: u64) -> bool {
        let removed = self
            .inner
            .lock()
            .expect("log subs mutex poisoned")
            .remove(&id);
        match removed {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    /// tail task 自然退出时自清理，避免登记表残留。
    pub fn finish(&self, id: u64) {
        self.inner
            .lock()
            .expect("log subs mutex poisoned")
            .remove(&id);
    }

    /// 当前活跃订阅数。
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("log subs mutex poisoned").len()
    }
}

impl Default for LogSubRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_monotonic() {
        let reg = LogSubRegistry::new();
        let (a, _) = reg.register();
        let (b, _) = reg.register();
        assert_ne!(a, b);
        assert!(b > a);
    }

    #[test]
    fn cancel_sets_flag_and_removes_entry() {
        let reg = LogSubRegistry::new();
        let (id, flag) = reg.register();
        assert_eq!(reg.len(), 1);
        assert!(!flag.load(Ordering::Relaxed));

        assert!(reg.cancel(id));
        assert!(flag.load(Ordering::Relaxed), "取消后标志必须置位");
        assert_eq!(reg.len(), 0);
        // 重复取消返回 false，不 panic
        assert!(!reg.cancel(id));
    }

    #[test]
    fn finish_removes_entry_without_setting_flag() {
        let reg = LogSubRegistry::new();
        let (id, flag) = reg.register();
        reg.finish(id);
        assert_eq!(reg.len(), 0);
        assert!(!flag.load(Ordering::Relaxed));
    }
}
