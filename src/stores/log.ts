// log store：日志面板状态机（阶段 6）。
//
// 两种模式：
//   live（实时）：通过 Channel 订阅当日日志，tail task 推送的 chunk 不断追加到 lines。
//   history（历史）：按日期分页读取，向前/向后翻页。
//
// Channel 生命周期（ADR-002）：
//   startLive 创建 Channel 并保存引用（channelRef），防 GC；stopLive 置 null 触发 GC，
//   后端 tail task 检测到 send 失败自动退出，无需显式 unsubscribe 命令。
//
// 约定：lines 在 live 模式累加追加；history 模式按翻页替换/追加。
// 切换模式前必须先 stopLive 释放旧订阅。

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { Channel } from '@tauri-apps/api/core'
import { ElMessage } from 'element-plus'
import { subscribeLog, readLogHistory, listLogDates, clearLog } from '@/api/log'
import type { LogChunk } from '@/types/log'
import { safeCall } from '@/api/invoke'

/** 历史分页单页字节上限（与后端 HISTORY_PAGE_LIMIT_MAX 对齐：256KB） */
const HISTORY_PAGE_SIZE = 256 * 1024

export type LogMode = 'live' | 'history'

export const useLogStore = defineStore('log', () => {
  /** 当前模式 */
  const mode = ref<LogMode>('live')
  /** 累积的日志文本（live 模式持续追加；history 模式按页拼接） */
  const lines = ref('')
  /** 是否加载中（首次订阅 / 历史翻页） */
  const loading = ref(false)
  /** 历史可用日期列表（YYYYMMDD，降序） */
  const dates = ref<string[]>([])
  /** 当前查看的历史日期 */
  const historyDate = ref<string>('')
  /** 历史分页当前偏移 */
  const historyOffset = ref(0)
  /** 历史是否还有下一页 */
  const historyHasMore = ref(false)

  /** 当前订阅的 Channel 引用（持有防 GC）。null 表示未订阅 */
  let channelRef: Channel<LogChunk> | null = null
  /** 当前订阅的项目 id（切换项目时需先 stopLive） */
  let liveProjectId: number | null = null

  // ===== 实时模式 =====

  /**
   * 开始实时订阅某项目的当日日志。
   * 幂等：若已订阅同一项目，直接返回；若订阅的是别的项目，先 stop 再 start。
   */
  async function startLive(projectId: number) {
    if (liveProjectId === projectId && channelRef) return
    stopLive()

    mode.value = 'live'
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

    const [, err] = await safeCall(() => subscribeLog(projectId, channel))
    loading.value = false
    if (err) {
      ElMessage.error(`订阅实时日志失败：${err}`)
      stopLive()
    }
  }

  /** 停止实时订阅（释放 Channel 引用 → GC → 后端自动退订） */
  function stopLive() {
    channelRef = null
    liveProjectId = null
  }

  // ===== 历史模式 =====

  /** 拉取历史日期列表 */
  async function fetchDates(projectId: number) {
    const [list, err] = await safeCall(() => listLogDates(projectId))
    if (err) {
      ElMessage.error(`读取日志日期失败：${err}`)
      return
    }
    dates.value = list ?? []
  }

  /**
   * 切换到历史模式并加载某日期首页。
   * @param reset true 表示重新读取（切换日期），把结果设为日志开头
   */
  async function loadHistory(projectId: number, date: string, reset: boolean) {
    // 进入历史模式必须先停实时订阅
    stopLive()
    mode.value = 'history'

    if (reset) {
      historyDate.value = date
      historyOffset.value = 0
      lines.value = ''
    }

    loading.value = true
    const [page, err] = await safeCall(() =>
      readLogHistory(projectId, date, historyOffset.value, HISTORY_PAGE_SIZE),
    )
    loading.value = false
    if (err) {
      ElMessage.error(`读取历史日志失败：${err}`)
      return
    }
    if (page) {
      if (reset) {
        lines.value = page.data
      } else {
        lines.value += page.data
      }
      historyOffset.value = page.next_offset ?? historyOffset.value
      historyHasMore.value = page.next_offset !== null
    }
  }

  /** 历史模式加载下一页（追加到现有内容后） */
  async function loadNextPage(projectId: number) {
    if (!historyHasMore.value || !historyDate.value) return
    await loadHistory(projectId, historyDate.value, false)
  }

  // ===== 清空 =====

  /** 清空当天日志文件（后端以 paths::today() 兜底日期） */
  async function clear(projectId: number) {
    const [, err] = await safeCall(() => clearLog(projectId))
    if (err) {
      ElMessage.error(`清空日志失败：${err}`)
      return false
    }
    // 清空按钮仅在 live 模式显示，同步清空展示
    if (mode.value === 'live') {
      lines.value = ''
    }
    return true
  }

  /** 重置全部状态（离开详情页时调用） */
  function reset() {
    stopLive()
    mode.value = 'live'
    lines.value = ''
    loading.value = false
    dates.value = []
    historyDate.value = ''
    historyOffset.value = 0
    historyHasMore.value = false
  }

  return {
    mode,
    lines,
    loading,
    dates,
    historyDate,
    historyHasMore,
    startLive,
    stopLive,
    fetchDates,
    loadHistory,
    loadNextPage,
    clear,
    reset,
  }
})
