// monitor 命令薄包装。
// 对齐 src-tauri/src/commands/process.rs::probe_statuses。

import type { ProjectStatus } from '@/types/monitor'
import { invokeCmd } from './invoke'

/**
 * 探测项目运行态（CPU/内存/端口）。
 *
 * @param ids 省略/空 → 探测全部运行中项目；传值则只探测给定 id（减少开销）
 * @returns 仅含运行中的项目；不在结果中的 id 视为 stopped
 */
export function probeStatuses(ids?: number[]): Promise<ProjectStatus[]> {
  return invokeCmd<ProjectStatus[]>('probe_statuses', { ids: ids ?? null })
}
