// cleaner store：全系统开发进程扫描 + 内存清理器状态。
//
// 与 project store 的区别：project 管理本工具**托管**的项目进程；
// cleaner 面向全系统**任意**开发进程（含 IDE 派生、孤立残留），用于释放内存。
//
// 轮询生命周期由 CleanerDrawer 控制：抽屉打开时 startPolling()，关闭时 stopPolling()。
// 杀进程操作不自动执行——所有清理都需用户确认后调用 killSelected()。

import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import { scanDevProcesses, killDevProcesses, getSystemMemory } from '@/api/cleaner'
import type { DevProcInfo, KillResult, SystemMemory } from '@/types/cleaner'
import { CATEGORY_META } from '@/types/cleaner'
import { safeCall } from '@/api/invoke'
import { usePolling } from '@/composables/usePolling'

/** 清理抽屉轮询间隔（ms）：比项目监控稍慢，5s 足够 */
export const CLEANER_POLL_MS = 5000

export const useCleanerStore = defineStore('cleaner', () => {
  /** 扫描到的开发进程列表（按 tree_memory_bytes 降序，后端已排） */
  const processes = ref<DevProcInfo[]>([])
  /** 系统内存概况 */
  const systemMem = ref<SystemMemory | null>(null)
  /** 是否正在扫描中（防止并发） */
  const scanning = ref(false)
  /** 是否正在执行清理 */
  const killing = ref(false)
  /** 已选中的进程 PID 集合（用于批量清理）。reactive 原生代理 Set，无需重新赋值触发响应。 */
  const selectedPids = reactive(new Set<number>())

  // ===== 扫描 =====

  /** 立即扫描一次（重入安全：上一轮未完成则跳过） */
  async function scan() {
    if (scanning.value) return
    scanning.value = true
    try {
      const [procs, mem] = await Promise.all([
        scanDevProcesses(),
        getSystemMemory(),
      ])
      processes.value = procs
      systemMem.value = mem
      // 清理已选中但已不存在的 PID（杀掉的进程应从选中集移除）
      const currentPids = new Set(procs.map((p) => p.pid))
      for (const pid of [...selectedPids]) {
        if (!currentPids.has(pid)) selectedPids.delete(pid)
      }
    } catch (e) {
      // 扫描失败不打断 UI；下次轮询重试
      console.warn('scan_dev_processes 失败:', e)
    } finally {
      scanning.value = false
    }
  }

  const { start: startPolling, stop: stopPolling } = usePolling(scan, CLEANER_POLL_MS)

  // ===== 选择 =====

  /** 某进程是否受保护（不可选中） */
  function isProtected(p: DevProcInfo): boolean {
    return CATEGORY_META[p.category]?.protected === true
  }

  /** 切换某进程的选中态（受保护进程忽略） */
  function toggleSelect(pid: number) {
    const p = processes.value.find((x) => x.pid === pid)
    if (!p || isProtected(p)) return
    if (selectedPids.has(pid)) selectedPids.delete(pid)
    else selectedPids.add(pid)
  }

  /** 选中/取消选中全部推荐项 */
  function selectAllRecommended() {
    const recs = recommendedPids.value
    // 若推荐项已全部选中 → 取消全部选中；否则全选推荐
    const allSelected = recs.every((pid) => selectedPids.has(pid))
    if (allSelected) {
      for (const pid of recs) selectedPids.delete(pid)
    } else {
      for (const pid of recs) selectedPids.add(pid)
    }
  }

  /** 清空选择 */
  function clearSelection() {
    selectedPids.clear()
  }

  // ===== 计算 =====

  /** 推荐清理的进程列表（共享过滤结果，避免重复遍历） */
  const recommendedProcs = computed(() =>
    processes.value.filter((p) => p.recommended && !isProtected(p)),
  )

  /** 推荐清理的 PID 列表 */
  const recommendedPids = computed(() => recommendedProcs.value.map((p) => p.pid))

  /** 推荐清理项的内存总和（字节） */
  const totalRecommendedMemory = computed(() =>
    recommendedProcs.value.reduce((sum, p) => sum + p.tree_memory_bytes, 0),
  )

  /** 全部进程的内存总和（字节） */
  const totalProcessMemory = computed(() =>
    processes.value.reduce((sum, p) => sum + p.tree_memory_bytes, 0),
  )

  /** 已选中项的内存总和（字节） */
  const selectedMemory = computed(() => {
    return processes.value
      .filter((p) => selectedPids.has(p.pid))
      .reduce((sum, p) => sum + p.tree_memory_bytes, 0)
  })

  /** 已选中项的 PID 树列表（传给后端 kill） */
  const selectedPidTrees = computed(() =>
    processes.value.filter((p) => selectedPids.has(p.pid)).map((p) => p.tree_pid_list),
  )

  // ===== 清理 =====

  /**
   * 杀掉所有已选中的进程树。
   * 返回 [result, error] 二元组；成功后刷新列表 + 清空选择。
   */
  async function killSelected(): Promise<[KillResult | null, string | null]> {
    if (selectedPidTrees.value.length === 0) {
      return [null, null]
    }
    killing.value = true
    try {
      const [result, error] = await safeCall(() =>
        killDevProcesses(selectedPidTrees.value),
      )
      if (error || !result) return [null, error]
      // 清理成功：清空选择，立即重新扫描确认释放效果
      clearSelection()
      void scan()
      return [result, null]
    } finally {
      killing.value = false
    }
  }

  /**
   * 杀掉单个进程树（行内「杀」按钮）。
   * 与 killSelected 共用后端，但只杀一棵树且不依赖全局选择集。
   */
  async function killOne(p: DevProcInfo): Promise<[KillResult | null, string | null]> {
    if (isProtected(p)) return [null, '受保护的 IDE 进程不可清理']
    killing.value = true
    try {
      const [result, error] = await safeCall(() => killDevProcesses([p.tree_pid_list]))
      if (error || !result) return [null, error]
      // 从选择集移除（若选中了）
      selectedPids.delete(p.pid)
      void scan()
      return [result, null]
    } finally {
      killing.value = false
    }
  }

  return {
    processes,
    systemMem,
    scanning,
    killing,
    selectedPids,
    recommendedPids,
    totalRecommendedMemory,
    totalProcessMemory,
    selectedMemory,
    selectedPidTrees,
    scan,
    startPolling,
    stopPolling,
    isProtected,
    toggleSelect,
    selectAllRecommended,
    clearSelection,
    killSelected,
    killOne,
  }
})
