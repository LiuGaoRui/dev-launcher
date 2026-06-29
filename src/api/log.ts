// log 命令薄包装。
// 对齐 src-tauri/src/commands/log.rs：subscribe_log（Channel 实时推送）/ clear_log。
//
// 日志按类型分 start（启动日志）/ build（构建日志），均为实时 tail。
// Channel 用法（ADR-002）：创建 Channel<LogChunk>，设置 onmessage 回调，
// 作为 onEvent 参数传入 invoke。前端持有 channel 引用防 GC（GC 后后端自动退订）。

import { Channel } from '@tauri-apps/api/core'
import type { LogChunk, LogType } from '@/types/log'
import { invokeCmd } from './invoke'

/**
 * 订阅实时日志。传入 Channel，后端 tail task 会持续把新增内容推到 channel.onmessage。
 *
 * 调用方必须持有 channel 引用直到不想再接收；释放引用触发 GC 后，
 * 后端 send() 失败、tail task 自动退出（无需显式 unsubscribe）。
 */
export function subscribeLog(
  projectId: number,
  logType: LogType,
  onEvent: Channel<LogChunk>,
): Promise<void> {
  return invokeCmd<void>('subscribe_log', { projectId, logType, onEvent })
}

/** 清空指定类型日志：truncate 保留文件本身 */
export function clearLog(projectId: number, logType: LogType): Promise<void> {
  return invokeCmd<void>('clear_log', { projectId, logType })
}
