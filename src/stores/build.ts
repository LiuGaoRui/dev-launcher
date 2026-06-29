// build store：多项目后台构建状态。
//
// 构建改后台执行：startBuild 调 build_project（立即返回），然后轮询 get_build_status
// 直到 running=false，把结果写入 states[id]。卡片读 states[id] 驱动构建按钮状态图标。
//
// 支持多项目同时构建：每个项目独立一份状态 + 独立轮询定时器。
// 构建输出实时写入 build.log，由日志页（详情页构建日志 Tab）订阅查看。

import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { buildProject, getBuildStatus } from '@/api/process'
import type { BuildState } from '@/types/project'
import { safeCall } from '@/api/invoke'
import { createDiscreteApi } from 'naive-ui'

const { message } = createDiscreteApi(['message'])

/** 状态轮询间隔（ms） */
const POLL_INTERVAL_MS = 1000

/** 默认状态（从未构建过的项目） */
function defaultState(): BuildState {
  return { running: false, exit_code: 0, duration_ms: 0, error: '' }
}

export const useBuildStore = defineStore('build', () => {
  /** 各项目构建状态：project_id → BuildState */
  const states = reactive<Record<number, BuildState>>({})

  /** 进行中的轮询定时器：project_id → 定时器句柄（停止构建/卸载时清理） */
  const timers: Record<number, ReturnType<typeof setInterval>> = {}

  /** 取某项目构建状态（无记录返回默认） */
  function getState(id: number): BuildState {
    return states[id] ?? defaultState()
  }

  /**
   * 触发后台构建：调 build_project（立即返回），开启状态轮询。
   * @param id 项目 id
   * @param onDone 可选：构建结束时回调（退出码作参数）
   */
  async function startBuild(id: number, onDone?: (exitCode: number) => void) {
    // 先乐观标记为构建中（让卡片立即转圈）
    states[id] = { running: true, exit_code: 0, duration_ms: 0, error: '' }

    const [, err] = await safeCall(() => buildProject(id))
    if (err) {
      // 命令本身失败（如未配 build_cmd、已在构建中）：回滚为失败状态
      states[id] = { running: false, exit_code: -1, duration_ms: 0, error: err }
      message.error(`启动构建失败：${err}`)
      onDone?.(-1)
      return
    }

    // 开启轮询，直到 running=false
    pollStatus(id, onDone)
  }

  /** 轮询某项目的构建状态，结束后停表 + 回调 */
  function pollStatus(id: number, onDone?: (exitCode: number) => void) {
    stopPoll(id)
    timers[id] = setInterval(async () => {
      const s = await getBuildStatus(id)
      states[id] = { ...s }
      if (!s.running) {
        stopPoll(id)
        if (s.error) {
          message.error(`构建失败：${s.error}`)
        } else if (s.exit_code !== 0) {
          message.error(`构建失败（退出码 ${s.exit_code}）`)
        }
        onDone?.(s.exit_code)
      }
    }, POLL_INTERVAL_MS)
  }

  /** 停止某项目的状态轮询（不杀构建进程，仅停止前端轮询） */
  function stopPoll(id: number) {
    const t = timers[id]
    if (t) {
      clearInterval(t)
      delete timers[id]
    }
  }

  /** 清空所有轮询定时器（卸载页面/离开时调用） */
  function clearAllTimers() {
    for (const key of Object.keys(timers)) {
      clearInterval(timers[Number(key)])
      delete timers[Number(key)]
    }
  }

  return {
    states,
    getState,
    startBuild,
    stopPoll,
    clearAllTimers,
  }
})
