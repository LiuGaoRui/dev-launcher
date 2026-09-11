// log 命令薄包装。
// 对齐 src-tauri/src/commands/log.rs：subscribe_log / unsubscribe_log / clear_log。
//
// 日志按类型分 start（启动日志）/ build（构建日志），均为实时 tail。
// 订阅生命周期（ADR-006）：subscribe_log 返回后端分配的 subscriptionId，
// 调用方切换订阅或卸载时必须调 unsubscribeLog 显式取消，不依赖 Channel GC。

import { Channel } from '@tauri-apps/api/core'
import type { LogChunk, LogType } from '@/types/log'
import { invokeCmd } from './invoke'

/**
 * 订阅实时日志，返回后端分配的订阅 id。
 *
 * 传入 Channel，后端 tail task 会持续把新增内容推到 channel.onmessage。
 * 返回的 subscriptionId 必须保存在调用方，停止订阅时传给 {@link unsubscribeLog}。
 */
export function subscribeLog(
  projectId: number,
  logType: LogType,
  onEvent: Channel<LogChunk>,
): Promise<number> {
  return invokeCmd<number>('subscribe_log', { projectId, logType, onEvent })
}

/** 取消日志订阅：后端 tail task 随即退出。幂等（id 不存在时后端返回成功） */
export function unsubscribeLog(subscriptionId: number): Promise<void> {
  return invokeCmd<void>('unsubscribe_log', { subscriptionId })
}

/** 清空指定类型日志：truncate 保留文件本身 */
export function clearLog(projectId: number, logType: LogType): Promise<void> {
  return invokeCmd<void>('clear_log', { projectId, logType })
}
