//! 进程树收集：基于 sysinfo 收集一个 PID 的全部后代。
//!
//! 用途（阶段 5 监控）：
//! - 端口归属校验：判断某端口占用者 PID 是否属于本项目进程树
//! - CPU/内存聚合：对整树求和
//!
//! sysinfo 无内建「后代」API，需遍历 `processes()` 按 `parent()` 反查。

use sysinfo::{Pid, System};

/// 收集 `root` PID 的全部后代 PID（不含 root 自身），递归。
///
/// 通过反复扫描「父 PID == 已知节点」的关系。进程树天然是树结构（无环），无需去重。
#[allow(dead_code)] // 阶段 5 监控（端口归属/CPU 聚合）使用
pub fn collect_descendants(root: u32, system: &System) -> Vec<u32> {
    let root_pid = Pid::from_u32(root);
    let mut all: Vec<u32> = Vec::new();
    let mut frontier = vec![root_pid];

    while let Some(current) = frontier.pop() {
        for (pid, proc) in system.processes() {
            if proc.parent() == Some(current) {
                let child = pid.as_u32();
                all.push(child);
                frontier.push(*pid);
            }
        }
    }
    all
}

/// 收集包含 root 自身的全树 PID（root 在首位）。
#[allow(dead_code)] // 阶段 5 监控使用
pub fn collect_tree(root: u32, system: &System) -> Vec<u32> {
    let mut tree = vec![root];
    tree.extend(collect_descendants(root, system));
    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_descendants_of_nonexistent_is_empty() {
        let system = System::new();
        // 0xFFFFFFFF 几乎不可能是真实 PID
        assert!(collect_descendants(0xFFFF_FFFF, &system).is_empty());
    }

    #[test]
    fn collect_tree_includes_root() {
        let system = System::new();
        let tree = collect_tree(0xFFFF_FFFF, &system);
        assert_eq!(tree, vec![0xFFFF_FFFF]);
    }
}
