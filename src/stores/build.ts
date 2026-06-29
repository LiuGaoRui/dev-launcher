// build store：多项目后台构建状态。
//
// 每个项目独立 Ref，支持多项目并发构建 + 跨路由持久化。
// 轮询使用递归 setTimeout（天然串行，不会回调叠加）。
//
// 构建改后台执行：startBuild 调 build_project（立即返回），然后轮询 get_build_status
// 直到 running=false。卡片通过 getState(id) 读取。

import { defineStore } from 'pinia'
import { ref, type Ref } from 'vue'
import { buildProject, getBuildStatus } from '@/api/process'
import type { BuildState } from '@/types/project'
import { safeCall } from '@/api/invoke'
import { createDiscreteApi } from 'naive-ui'

const { message } = createDiscreteApi(['message'])

const POLL_INTERVAL_MS = 1000

function defaultState(): BuildState {
  return { running: false, exit_code: 0, duration_ms: 0, error: '' }
}

interface BuildEntry {
  state: Ref<BuildState>
  timer: ReturnType<typeof setTimeout> | null
}

export const useBuildStore = defineStore('build', () => {
  const entries: Record<number, BuildEntry> = {}

  function ensureEntry(id: number): BuildEntry {
    if (!entries[id]) {
      entries[id] = { state: ref(defaultState()), timer: null }
    }
    return entries[id]
  }

  function getState(id: number): BuildState {
    return ensureEntry(id).state.value
  }

  async function startBuild(id: number, onDone?: (exitCode: number) => void) {
    const entry = ensureEntry(id)
    entry.state.value = { running: true, exit_code: 0, duration_ms: 0, error: '' }

    const [, err] = await safeCall(() => buildProject(id))
    if (err) {
      entry.state.value = { running: false, exit_code: -1, duration_ms: 0, error: err }
      message.error(`启动构建失败：${err}`)
      onDone?.(-1)
      return
    }

    pollStatus(id, onDone)
  }

  function pollStatus(id: number, onDone?: (exitCode: number) => void) {
    stopPoll(id)

    const tick = async () => {
      const entry = entries[id]
      if (!entry) return
      try {
        const s = await getBuildStatus(id)
        entry.state.value = { ...s }
        if (!s.running) {
          stopPoll(id)
          if (s.error) {
            message.error(`构建失败：${s.error}`)
          } else if (s.exit_code !== 0) {
            message.error(`构建失败（退出码 ${s.exit_code}）`)
          }
          onDone?.(s.exit_code)
          return
        }
      } catch {
        // IPC 偶发失败，跳过本次轮询，下一 tick 重试
      }
      if (entries[id]) {
        entries[id].timer = setTimeout(tick, POLL_INTERVAL_MS)
      }
    }

    ensureEntry(id).timer = setTimeout(tick, POLL_INTERVAL_MS)
  }

  function stopPoll(id: number) {
    const entry = entries[id]
    if (entry) {
      if (entry.timer) clearTimeout(entry.timer)
      delete entries[id]
    }
  }

  function clearAllTimers() {
    for (const key of Object.keys(entries)) {
      const entry = entries[Number(key)]
      if (entry.timer) clearTimeout(entry.timer)
      delete entries[Number(key)]
    }
  }

  return { getState, startBuild, stopPoll, clearAllTimers }
})
