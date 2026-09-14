// log store：实时日志订阅（启动日志 / 构建日志）。
//
// 每次启动/构建前由后端 truncate，日志文件只含本次输出，故只有实时模式。
// 两种日志类型通过 logType 区分：start（启动日志）/ build（构建日志）。
//
// 订阅隔离（ADR-006，修复「串日志」）：
//   历史上 stopLive 只把 Channel 引用置 null，靠 GC 触发后端退订；GC 时机不确定，
//   旧 tail task 在窗口期内仍会把旧项目/旧类型的日志追加进新的共享缓冲。
//   现在每次 startLive 递增 token，onmessage 里校验 token 与 chunk.sub_id：
//   过期订阅的推送一律丢弃；stopLive 同时显式调 unsubscribe_log 让后端立即退出。

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { Channel } from '@tauri-apps/api/core'
import { createDiscreteApi } from 'naive-ui'
import { subscribeLog, unsubscribeLog, clearLog } from '@/api/log'
import type { LogChunk, LogType } from '@/types/log'
import { safeCall } from '@/api/invoke'

const { message } = createDiscreteApi(['message'])

export const useLogStore = defineStore('log', () => {
  /** 累积的日志文本（实时追加，仅属于当前活跃订阅，超过 MAX_LINES 行 / MAX_CHARS 字符截断头部） */
  const lines = ref('')
  /** 当前行数（增量维护；语义与「非空文本 = 换行符数 + 1」一致） */
  const lineCount = ref(0)
  /** 是否加载中（首次订阅） */
  const loading = ref(false)
  /** 当前订阅的日志类型 */
  const currentType = ref<LogType>('start')

  /** 缓冲上限（行）：超过后丢弃头部整行。全量日志在磁盘文件中，UI 仅保留尾部（ADR-007） */
  const MAX_LINES = 5000
  /**
   * 缓冲硬上限（字符）：行数截断只对含换行符的输出生效，无换行的病态输出
   * （如 \r 刷新的进度条、单行压缩文本）需此上限无视行对齐兜底，否则缓冲
   * 无界增长会撞上 128MB V8 old space 上限导致 renderer OOM。取值需满足
   * 「正常 5000 行日志不触发」（约 10MB UTF-16，远低于 old space 上限）。
   */
  const MAX_CHARS = 5_000_000

  /** 订阅序号：每次 start/stop 递增，用于作废所有旧回调 */
  let seq = 0
  /** 当前有效 token；onmessage 只在 token 相等时写入 lines */
  let activeToken = 0
  /** 当前活跃订阅的后端 id；null 表示未订阅 */
  let activeSubId: number | null = null
  /** 当前活跃订阅的项目 id / 类型（幂等判断用） */
  let activeProjectId: number | null = null
  let activeType: LogType | null = null

  /**
   * 追加一段日志并维持缓冲有界：先按行数截断（超过 MAX_LINES 行从头部丢弃
   * 整行），再用字符硬上限兜底（无换行的病态输出）。行数增量维护，
   * 避免每次推送 O(n) 全扫。
   */
  function appendChunk(text: string) {
    if (!text) return
    let next = lines.value + text
    let newlines = 0
    for (let i = text.indexOf('\n'); i !== -1; i = text.indexOf('\n', i + 1)) newlines++
    let count = lines.value === '' ? newlines + 1 : lineCount.value + newlines
    if (count > MAX_LINES) {
      const drop = count - MAX_LINES
      // 丢掉前 drop 行 = 连同第 drop 个换行符一起丢弃；找不齐换行符（超长无换行）则交给字符上限兜底
      let idx = -1
      for (let i = 0; i < drop; i++) {
        const hit = next.indexOf('\n', idx + 1)
        if (hit === -1) {
          idx = -1
          break
        }
        idx = hit
      }
      if (idx !== -1) {
        next = next.slice(idx + 1)
        count -= drop
      }
    }
    if (next.length > MAX_CHARS) {
      // 字符兜底：无视行对齐保留尾部 MAX_CHARS 字符，行数按丢弃段内的换行符数扣减
      const cut = next.length - MAX_CHARS
      let dropped = 0
      for (let i = next.indexOf('\n'); i !== -1 && i < cut; i = next.indexOf('\n', i + 1)) dropped++
      next = next.slice(cut)
      count -= dropped
    }
    lines.value = next
    lineCount.value = count
  }

  /** 清空本地缓冲（订阅切换 / 清空日志 / 重置时） */
  function resetBuffer() {
    lines.value = ''
    lineCount.value = 0
  }

  /**
   * 开始实时订阅某项目指定类型的日志。
   * 幂等：若已订阅同一项目+同一类型，直接返回；否则先停掉旧订阅再起新的。
   */
  async function startLive(projectId: number, logType: LogType) {
    if (activeProjectId === projectId && activeType === logType && activeSubId !== null) return

    stopLive()

    // 认领本次订阅：此后旧 Channel 的所有 onmessage 都会被 token 校验拦下
    const myToken = ++seq
    activeToken = myToken
    activeProjectId = projectId
    activeType = logType
    currentType.value = logType
    resetBuffer()
    loading.value = true

    const channel = new Channel<LogChunk>()
    channel.onmessage = (chunk: LogChunk) => {
      // 双重校验：token 必须是当前订阅；sub_id 已就绪时必须匹配本订阅。
      // sub_id 尚未返回时放行（后端从 offset 0 推送的首批内容早于 invoke 返回）。
      if (activeToken !== myToken) return
      if (activeSubId !== null && chunk.sub_id !== activeSubId) return
      appendChunk(chunk.text)
    }

    const [subId, err] = await safeCall(() => subscribeLog(projectId, logType, channel))

    if (activeToken !== myToken) {
      // 期间已切换/停止：退订这次迟到返回的订阅，且不触碰任何共享状态
      if (subId !== null) {
        safeCall(() => unsubscribeLog(subId)).then(([, e]) => {
          if (e) console.warn(`退订过期日志订阅失败(sub ${subId})：${e}`)
        })
      }
      return
    }

    loading.value = false
    if (err !== null || subId === null) {
      message.error(`订阅实时日志失败：${err ?? '未知错误'}`)
      activeToken = ++seq // 作废本次 token，避免残留回调写入
      activeSubId = null
      activeProjectId = null
      activeType = null
      return
    }
    activeSubId = subId
  }

  /**
   * 停止订阅：同步作废 token（旧回调立即失效）+ 显式通知后端退订。
   * 不 await 退订请求：状态已在本地失效，后端取消有 250ms 内的延迟也无影响。
   */
  function stopLive() {
    activeToken = ++seq
    const subId = activeSubId
    activeSubId = null
    activeProjectId = null
    activeType = null
    if (subId !== null) {
      safeCall(() => unsubscribeLog(subId)).then(([, e]) => {
        if (e) console.warn(`取消日志订阅失败(sub ${subId})：${e}`)
      })
    }
  }

  /** 清空指定类型的日志文件（后端 truncate） */
  async function clear(projectId: number, logType: LogType) {
    const [, err] = await safeCall(() => clearLog(projectId, logType))
    if (err) {
      message.error(`清空日志失败：${err}`)
      return false
    }
    // 仅当清空的正是当前展示的日志时同步清本地缓冲
    if (activeProjectId === projectId && activeType === logType) {
      resetBuffer()
    }
    return true
  }

  /** 重置全部状态（离开详情页时调用） */
  function reset() {
    stopLive()
    resetBuffer()
    loading.value = false
    currentType.value = 'start'
  }

  return {
    lines,
    lineCount,
    loading,
    currentType,
    startLive,
    stopLive,
    clear,
    reset,
  }
})
