//! 进程树收集：基于 sysinfo 收集一个 PID 的全部后代。
//!
//! 用途（阶段 5 监控）：
//! - 端口归属校验：判断某端口占用者 PID 是否属于本项目进程树
//! - CPU/内存聚合：对整树求和
//!
//! sysinfo 无内建「后代」API，需遍历 `processes()` 按 `parent()` 反查。

use std::collections::HashMap;

use sysinfo::{Pid, System};

/// 构建 parent_pid → [child_pid] 索引（只含存在父进程的进程）。
///
/// 供批量调用 `collect_descendants_with_map` 时复用，避免每个 root 都扫描全进程表。
pub fn build_parent_map(system: &System) -> HashMap<u32, Vec<u32>> {
    let mut map: HashMap<u32, Vec<u32>> = HashMap::new();
    for (pid, proc) in system.processes() {
        if let Some(parent) = proc.parent() {
            map.entry(parent.as_u32()).or_default().push(pid.as_u32());
        }
    }
    map
}

/// 收集 `root` PID 的全部后代 PID（不含 root 自身），基于预建的 parent→children 索引。
pub fn collect_descendants_with_map(root: u32, map: &HashMap<u32, Vec<u32>>) -> Vec<u32> {
    let mut all: Vec<u32> = Vec::new();
    let mut frontier = vec![root];
    while let Some(current) = frontier.pop() {
        if let Some(children) = map.get(&current) {
            for &child in children {
                all.push(child);
                frontier.push(child);
            }
        }
    }
    all
}

/// 收集 `root` PID 的全部后代 PID（不含 root 自身），递归。
///
/// 通过反复扫描「父 PID == 已知节点」的关系。进程树天然是树结构（无环），无需去重。
/// 单次调用场景用此函数；批量场景用 `build_parent_map` + `collect_descendants_with_map`。
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
pub fn collect_tree(root: u32, system: &System) -> Vec<u32> {
    let mut tree = vec![root];
    tree.extend(collect_descendants(root, system));
    tree
}

/// 对全树 PID 聚合 CPU（f32 求和）/ 内存（bytes 求和）。
///
/// CPU 说明：sysinfo `cpu_usage()` 为「自上次 refresh 以来的平均」，
/// 整树求和后由调用方除以逻辑核数归一化。
pub fn aggregate_tree(tree_pids: &[u32], system: &System) -> (f32, u64) {
    let mut cpu = 0.0f32;
    let mut mem = 0u64;
    for &pid_u32 in tree_pids {
        if let Some(proc) = system.process(Pid::from_u32(pid_u32)) {
            cpu += proc.cpu_usage();
            mem += proc.memory();
        }
    }
    (cpu, mem)
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
