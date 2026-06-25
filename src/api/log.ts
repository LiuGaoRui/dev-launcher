// log 命令薄包装。
// 对齐 src-tauri/src/commands/log.rs：
//   subscribe_log（Channel 实时推送）/ read_log_history（分页）/ list_log_dates / clear_log。
//
// Channel 用法（ADR-002）：创建 Channel<LogChunk>，设置 onmessage 回调，
// 作为 onEvent 参数传入 invoke。前端持有 channel 引用防 GC（GC 后后端自动退订）。

import { Channel, invoke } from '@tauri-apps/api/core'
import type { LogChunk, LogHistoryPage } from '@/types/log'

/**
 * 订阅实时日志。传入 Channel，后端 tail task 会持续把新增内容推到 channel.onmessage。
 *
 * 调用方必须持有 channel 引用直到不想再接收；释放引用触发 GC 后，
 * 后端 send() 失败、tail task 自动退出（无需显式 unsubscribe）。
 */
export function subscribeLog(
  projectId: number,
  onEvent: Channel<LogChunk>,
): Promise<void> {
  return invoke<void>('subscribe_log', { projectId, onEvent })
}

/** 历史日志分页读取：从 date（YYYYMMDD）日志的 offset 处读 limit 字节 */
export function readLogHistory(
  projectId: number,
  date: string,
  offset: number,
  limit: number,
): Promise<LogHistoryPage> {
  return invoke<LogHistoryPage>('read_log_history', { projectId, date, offset, limit })
}

/** 列出某项目的全部历史日志日期（降序，最新在前） */
export function listLogDates(projectId: number): Promise<string[]> {
  return invoke<string[]>('list_log_dates', { projectId })
}

/** 清空日志：date 不传则清当天。truncate 保留文件本身 */
export function clearLog(projectId: number, date?: string): Promise<void> {
  return invoke<void>('clear_log', { projectId, date: date ?? null })
}
