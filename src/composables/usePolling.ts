// 通用轮询生命周期组合式函数。
// 提供幂等的 start/stop，由调用方传入轮询函数和间隔。
// 调用方负责在 onMounted/onBeforeUnmount 或 watch 中管理生命周期。

export function usePolling(fn: () => Promise<void>, intervalMs: number) {
  let timer: ReturnType<typeof setInterval> | null = null

  /** 立即执行一次 + 启动定时轮询（幂等：已运行则忽略） */
  function start() {
    if (timer !== null) return
    void fn()
    timer = setInterval(() => void fn(), intervalMs)
  }

  /** 停止轮询 */
  function stop() {
    if (timer !== null) {
      clearInterval(timer)
      timer = null
    }
  }

  return { start, stop }
}
