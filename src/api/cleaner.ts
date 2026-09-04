// cleaner 命令薄包装。
// 对齐 src-tauri/src/commands/cleaner.rs：scan_dev_processes / kill_dev_processes /
// get_system_memory + 进程锁定 CRUD（list/add/remove/clear_cleaner_locks）。

import { invokeCmd } from './invoke'
import type {
  CleanerLock,
  CleanerLockInput,
  DevProcInfo,
  KillResult,
  SystemMemory,
  TrimResult,
} from '@/types/cleaner'

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

// ===== 进程锁定（指纹持久化，与 PID 无关） =====

/** 列出全部锁定条目（含未运行的进程，按锁定时间倒序） */
export function listCleanerLocks(): Promise<CleanerLock[]> {
  return invokeCmd<CleanerLock[]>('list_cleaner_locks')
}

/**
 * 锁定一个进程（按指纹去重，重复锁定幂等）。
 * input 字段取自扫描结果 DevProcInfo 的指纹字段，后端规范化为匹配键。
 */
export function addCleanerLock(input: CleanerLockInput): Promise<CleanerLock> {
  return invokeCmd<CleanerLock>('add_cleaner_lock', { input })
}

/** 解除单条进程锁定（lockId 来自 DevProcInfo.lock_id） */
export function removeCleanerLock(lockId: number): Promise<void> {
  return invokeCmd<void>('remove_cleaner_lock', { lockId })
}

/** 清空全部进程锁定条目（「解锁全部」） */
export function clearCleanerLocks(): Promise<void> {
  return invokeCmd<void>('clear_cleaner_locks')
}
