// process 命令薄包装。
// 对齐 src-tauri/src/commands/process.rs：start_project / stop_project / restart_project。

import type { StartResult } from '@/types/project'
import { invokeCmd } from './invoke'

/** 启动项目，返回 root pid / 日志路径 / 启动时间 */
export function startProject(id: number): Promise<StartResult> {
  return invokeCmd<StartResult>('start_project', { id })
}

/** 停止项目（Job terminate 杀整树 → 轮询归零 → 兜底 kill） */
export function stopProject(id: number): Promise<void> {
  return invokeCmd<void>('stop_project', { id })
}

/** 重启项目（stop 若在运行 → 确认退出 → start） */
export function restartProject(id: number): Promise<StartResult> {
  return invokeCmd<StartResult>('restart_project', { id })
}
