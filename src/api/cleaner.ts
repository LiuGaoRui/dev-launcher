// cleaner 命令薄包装。
// 对齐 src-tauri/src/commands/cleaner.rs：scan_dev_processes / kill_dev_processes / get_system_memory。

import { invokeCmd } from './invoke'
import type { DevProcInfo, KillResult, SystemMemory } from '@/types/cleaner'

/**
 * 扫描全系统开发进程（java/javaw/node 等），返回智能分类后的列表。
 *
 * 后端复用持久化 sysinfo System 实例刷新全进程后扫描，排除本应用自身进程。
 */
export function scanDevProcesses(): Promise<DevProcInfo[]> {
  return invokeCmd<DevProcInfo[]>('scan_dev_processes')
}

/**
 * 批量杀进程树。
 *
 * @param pidTrees 每个元素是一棵进程树的 PID 列表（来自 DevProcInfo.tree_pid_list）。
 *   后端取每棵树的首个 PID，用 `taskkill /F /T` 递归杀整树。
 *   返回 KillResult { killed, failed, freed_bytes }。单棵树失败不中断其余。
 */
export function killDevProcesses(pidTrees: number[][]): Promise<KillResult> {
  return invokeCmd<KillResult>('kill_dev_processes', { pidTrees })
}

/**
 * 查询系统物理内存概况（总量/已用/可用/使用率）。
 *
 * 供清理抽屉顶部内存条展示。
 */
export function getSystemMemory(): Promise<SystemMemory> {
  return invokeCmd<SystemMemory>('get_system_memory')
}
