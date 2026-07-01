// cleaner 命令薄包装。
// 对齐 src-tauri/src/commands/cleaner.rs：scan_dev_processes / kill_dev_processes / get_system_memory。

import { invokeCmd } from './invoke'
import type { DevProcInfo, KillResult, SystemMemory, TrimResult } from '@/types/cleaner'

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
 * 批量修剪进程工作集（内存回收，不杀进程）。
 *
 * 对每个 PID 调用 SetProcessWorkingSetSizeEx(-1,-1)，请求 OS 把进程不活跃的
 * 物理页换出到 pagefile，立即降低 RSS。进程继续运行，下次访问时按需换入。
 *
 * 适用 IDE（IDEA/VSCode）等不能杀但内存高的进程。返回回收前后 RSS 差值。
 *
 * @param pids 要回收内存的进程 PID 列表（扁平 PID，非进程树）
 */
export function trimDevProcesses(pids: number[]): Promise<TrimResult> {
  return invokeCmd<TrimResult>('trim_dev_processes', { pids })
}

/**
 * 查询系统物理内存概况（总量/已用/可用/使用率）。
 *
 * 供清理抽屉顶部内存条展示。
 */
export function getSystemMemory(): Promise<SystemMemory> {
  return invokeCmd<SystemMemory>('get_system_memory')
}
