// process 命令薄包装。
// 对齐 src-tauri/src/commands/process.rs：
//   start_project / stop_project / build_project。

import { Channel } from '@tauri-apps/api/core'
import type { BuildEvent, BuildResult, StartResult } from '@/types/project'
import { invokeCmd } from './invoke'

/** 启动项目，返回 root pid / 日志路径 / 启动时间 */
export function startProject(id: number): Promise<StartResult> {
  return invokeCmd<StartResult>('start_project', { id })
}

/** 停止项目（Job terminate 杀整树 → 轮询归零 → 兜底 kill） */
export function stopProject(id: number): Promise<void> {
  return invokeCmd<void>('stop_project', { id })
}

/**
 * 构建项目：执行 build_cmd，stdout/stderr 实时推 Channel，跑完返回退出码。
 * 调用方须持有 channel 引用直到不想再接收；释放引用触发 GC 后后端读取 task 退出。
 */
export function buildProject(
  id: number,
  onEvent: Channel<BuildEvent>,
): Promise<BuildResult> {
  return invokeCmd<BuildResult>('build_project', { id, onEvent })
}
