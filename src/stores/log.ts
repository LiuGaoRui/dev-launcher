// log store：实时日志订阅（启动日志 / 构建日志）。
//
// 历史模式已移除（每次启动/构建前由后端 truncate，日志文件只含本次输出）。
// 两种日志类型通过 logType 区分：start（启动日志）/ build（构建日志）。
//
// Channel 生命周期（ADR-002）：
//   startLive 创建 Channel 并保存引用（channelRef），防 GC；stopLive 置 null 触发 GC，
//   后端 tail task 检测到 send 失败自动退出，无需显式 unsubscribe 命令。

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { Channel } from '@tauri-apps/api/core'
import { createDiscreteApi } from 'naive-ui'
import { subscribeLog, clearLog } from '@/api/log'
import type { LogChunk, LogType } from '@/types/log'
import { safeCall } from '@/api/invoke'

const { message } = createDiscreteApi(['message'])

export const useLogStore = defineStore('log', () => {
  /** 累积的日志文本（实时追加） */
  const lines = ref('')
  /** 是否加载中（首次订阅） */
  const loading = ref(false)
  /** 当前订阅的日志类型 */
  const currentType = ref<LogType>('start')

  /** 当前订阅的 Channel 引用（持有防 GC）。null 表示未订阅 */
  let channelRef: Channel<LogChunk> | null = null
  /** 当前订阅的项目 id（切换项目/类型时需先 stopLive） */
  let liveProjectId: number | null = null
  let liveType: LogType | null = null

  /**
   * 开始实时订阅某项目指定类型的日志。
   * 幂等：若已订阅同一项目+同一类型，直接返回；否则先 stop 再 start。
   */
  async function startLive(projectId: number, logType: LogType) {
    if (liveProjectId === projectId && liveType === logType && channelRef) return
    stopLive()

    currentType.value = logType
    lines.value = ''
    loading.value = true

    const channel = new Channel<LogChunk>()
    channel.onmessage = (chunk: LogChunk) => {
      // tail task 推送增量，追加到末尾
      lines.value += chunk.text
    }
    // 持有引用防 GC（GC 后后端自动退订）
    channelRef = channel
    liveProjectId = projectId
    liveType = logType

    const [, err] = await safeCall(() => subscribeLog(projectId, logType, channel))
    loading.value = false
    if (err) {
      message.error(`订阅实时日志失败：${err}`)
      stopLive()
    }
  }

  /** 停止订阅（释放 Channel 引用 → GC → 后端自动退订） */
  function stopLive() {
    channelRef = null
    liveProjectId = null
    liveType = null
  }

  /** 清空当前类型的日志文件（后端 truncate） */
  async function clear(projectId: number) {
    const [, err] = await safeCall(() => clearLog(projectId, currentType.value))
    if (err) {
      message.error(`清空日志失败：${err}`)
      return false
    }
    lines.value = ''
    return true
  }

  /** 重置全部状态（离开详情页时调用） */
  function reset() {
    stopLive()
    lines.value = ''
    loading.value = false
    currentType.value = 'start'
  }

  return {
    lines,
    loading,
    currentType,
    startLive,
    stopLive,
    clear,
    reset,
  }
})
