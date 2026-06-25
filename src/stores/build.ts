// build store：构建状态机（阶段 7）。
//
// 职责：创建 Channel 订阅 build_project 的实时输出（stdout/stderr），
// 按 kind 累积到 output；记录运行态、退出码、耗时、退出标志。
//
// Channel 生命周期（ADR-002）：startBuild 持有 channelRef 防 GC；
// stopBuild 置 null 触发 GC，后端读取 task send 失败退出、child 由 kill_on_drop 回收。
//
// 调用约定：build_project 是 await 命令（跑到结束才 resolve），
// 但实时输出通过 Channel 流式到达，故 startBuild 内部 fire-and-forget 调用，
// 通过 onDone 回调通知完成（退出码已随 Exit 事件到达，命令 resolve 仅用于错误兜底）。

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { Channel } from '@tauri-apps/api/core'
import { buildProject } from '@/api/process'
import type { BuildEvent, BuildResult } from '@/types/project'

/** 单条构建输出行（带来源标记，便于前端着色区分 stderr） */
export interface BuildLine {
  kind: 'stdout' | 'stderr'
  text: string
}

export const useBuildStore = defineStore('build', () => {
  /** 当前构建的项目 id */
  const projectId = ref<number | null>(null)
  /** 构建输出（按到达顺序，stdout/stderr 混合） */
  const lines = ref<BuildLine[]>([])
  /** 是否构建中 */
  const running = ref(false)
  /** 退出码（null=尚未退出） */
  const exitCode = ref<number | null>(null)
  /** 耗时（ms，退出后填） */
  const durationMs = ref<number | null>(null)
  /** 错误信息（命令本身抛错时填，如未配 build_cmd） */
  const error = ref('')

  /** 当前订阅的 Channel 引用（持有防 GC）。读取无意义，仅作为生命周期锚点保留 */
  const channelRef = ref<Channel<BuildEvent> | null>(null)
  /** 构建命令的 Promise（用于 abort/感知），实际不可中断，仅用于状态跟踪 */
  let buildPromise: Promise<BuildResult> | null = null
  /** 完成回调（一键发布编排用它知道构建结束） */
  let doneResolver: ((r: BuildResult | null) => void) | null = null

  /**
   * 启动构建。fire-and-forget 调用 buildProject，输出通过 Channel 流式到达。
   * @returns Promise<BuildResult|null> resolve 时构建已结束（成功返回结果，失败返回 null）
   */
  async function startBuild(id: number): Promise<BuildResult | null> {
    // 若上一个构建还在进行，先释放（同一时刻只支持一个构建面板）
    stopBuild()

    projectId.value = id
    lines.value = []
    running.value = true
    exitCode.value = null
    durationMs.value = null
    error.value = ''

    const channel = new Channel<BuildEvent>()
    channel.onmessage = (ev: BuildEvent) => {
      if (ev.kind === 'exit') {
        exitCode.value = ev.data
      } else {
        lines.value.push({ kind: ev.kind, text: ev.data })
      }
    }
    channelRef.value = channel

    // 返回一个在构建结束时 resolve 的 Promise（供编排等待）
    const done = new Promise<BuildResult | null>((resolve) => {
      doneResolver = resolve
    })

    // fire-and-forget：build_project 会 await 到进程退出
    buildPromise = buildProject(id, channel)
    buildPromise
      .then((r) => {
        durationMs.value = r.duration_ms
        running.value = false
        doneResolver?.(r)
        doneResolver = null
        return r
      })
      .catch((e) => {
        error.value = typeof e === 'string' ? e : (e as Error)?.message ?? String(e)
        running.value = false
        doneResolver?.(null)
        doneResolver = null
      })

    return done
  }

  /**
   * 停止构建订阅（置 null 触发 GC → 后端读取 task 退出 → kill_on_drop 回收 child）。
   * 注意：这是「放弃订阅」，不保证子进程立即终止（依赖后端 kill_on_drop）。
   */
  function stopBuild() {
    channelRef.value = null
    buildPromise = null
    doneResolver = null
    running.value = false
  }

  /** 重置全部状态（关闭对话框时调用） */
  function reset() {
    stopBuild()
    projectId.value = null
    lines.value = []
    exitCode.value = null
    durationMs.value = null
    error.value = ''
  }

  /** 构建是否成功结束（退出码 0） */
  function isSucceeded(): boolean {
    return exitCode.value === 0
  }

  return {
    projectId,
    lines,
    running,
    exitCode,
    durationMs,
    error,
    startBuild,
    stopBuild,
    reset,
    isSucceeded,
  }
})
