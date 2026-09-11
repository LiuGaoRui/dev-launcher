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
  /** 累积的日志文本（实时追加，仅属于当前活跃订阅） */
  const lines = ref('')
  /** 是否加载中（首次订阅） */
  const loading = ref(false)
  /** 当前订阅的日志类型 */
  const currentType = ref<LogType>('start')

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
    lines.value = ''
    loading.value = true

    const channel = new Channel<LogChunk>()
    channel.onmessage = (chunk: LogChunk) => {
      // 双重校验：token 必须是当前订阅；sub_id 已就绪时必须匹配本订阅。
      // sub_id 尚未返回时放行（后端从 offset 0 推送的首批内容早于 invoke 返回）。
      if (activeToken !== myToken) return
      if (activeSubId !== null && chunk.sub_id !== activeSubId) return
      lines.value += chunk.text
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
      lines.value = ''
    }
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
