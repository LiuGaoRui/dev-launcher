// cleaner store：全系统开发进程扫描 + 内存清理器状态。
//
// 与 project store 的区别：project 管理本工具**托管**的项目进程；
// cleaner 面向全系统**任意**开发进程（含 IDE 派生、孤立残留），用于释放内存。
//
// 轮询生命周期由 CleanerDrawer 控制：抽屉打开时 startPolling()，关闭时 stopPolling()。
// 杀进程操作不自动执行——所有清理都需用户确认后调用 killSelected()。
//
// 锁定机制：用户可锁定某些进程，锁定后该进程不可被选中、不可被清理/回收。
// 锁定按进程指纹（cmdline 优先，cwd+name 兜底）持久化到后端 cleaner_lock 表，
// 与 PID 无关——进程重启后 PID 变化不影响锁定，条目永不自动丢失；
// 进程未运行时条目也保留，仅「解锁全部」可清空。
// 指纹相同的多个进程实例会被一并锁定/解锁（指纹无法区分实例）。

import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import {
  scanDevProcesses,
  killDevProcesses,
  trimDevProcesses,
  getSystemMemory,
  listCleanerLocks,
  addCleanerLock,
  removeCleanerLock,
  clearCleanerLocks,
} from '@/api/cleaner'
import type {
  CleanerLockInput,
  DevProcInfo,
  KillResult,
  SystemMemory,
  TrimResult,
} from '@/types/cleaner'
import { CATEGORY_META } from '@/types/cleaner'
import { safeCall } from '@/api/invoke'
import { usePolling } from '@/composables/usePolling'

/** 清理抽屉轮询间隔（ms）：比项目监控稍慢，5s 足够 */
export const CLEANER_POLL_MS = 5000

// 旧版把锁定 PID 存 localStorage，进程重启后即失效（且被轮询清理逻辑清除），
// 已被后端指纹锁定取代。残留 key 一次性清除（旧 PID 无法迁移为指纹）。
try {
  localStorage.removeItem('devlauncher:cleaner:lockedPids')
} catch {
  // localStorage 不可用时忽略
}

export const useCleanerStore = defineStore('cleaner', () => {
  /** 扫描到的开发进程列表（按 tree_memory_bytes 降序，后端已排） */
  const processes = ref<DevProcInfo[]>([])
  /** 系统内存概况 */
  const systemMem = ref<SystemMemory | null>(null)
  /** 是否正在扫描中（防止并发） */
  const scanning = ref(false)
  /** 是否正在执行清理（杀进程） */
  const killing = ref(false)
  /** 是否正在执行内存回收（修剪工作集） */
  const trimming = ref(false)
  /** 是否允许清理 IDE 进程（危险模式，关闭 IDE 保护）。默认关闭，不持久化。 */
  const allowKillIde = ref(false)
  /** 已选中的进程 PID 集合（用于批量清理）。reactive 原生代理 Set，无需重新赋值触发响应。 */
  const selectedPids = reactive(new Set<number>())
  /**
   * 锁定条目总数（cleaner_lock 表，含未运行进程的条目）。
   * 「解锁全部」按此计数清空所有条目；单个进程的锁定态看 DevProcInfo.locked。
   */
  const lockedCount = ref(0)

  /** 从后端拉取锁定条目总数（store 初始化时加载一次，增删锁定时本地维护） */
  async function refreshLockCount() {
    const [locks, err] = await safeCall(() => listCleanerLocks())
    if (!err && locks) lockedCount.value = locks.length
  }
  void refreshLockCount()

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
      // 清理已选中但已不存在的 PID（杀掉的进程应从选中集移除）。
      // 注意：不清理锁定——锁定按指纹持久化在库中，进程不在运行也保留。
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

  /** 某进程是否受保护（不可选中）。开启 allowKillIde 后 IDE 保护失效。 */
  function isProtected(p: DevProcInfo): boolean {
    if (allowKillIde.value) return false
    return CATEGORY_META[p.category]?.protected === true
  }

  /** 某进程是否被用户锁定（不可选中、不可清理、不可回收）。
   *  锁定态由后端按指纹匹配后随扫描结果返回（DevProcInfo.locked）。 */
  function isLocked(p: DevProcInfo): boolean {
    return p.locked
  }

  /** 从 DevProcInfo 提取锁定指纹输入（与后端 CleanerLockInput 对齐，snake_case） */
  function toLockInput(p: DevProcInfo): CleanerLockInput {
    return {
      fingerprint: p.fingerprint,
      name: p.name,
      cmdline: p.cmdline,
      cwd: p.cwd,
      exe: p.exe,
      display_title: p.display_title,
      cmdline_summary: p.cmdline_summary,
      pid: p.pid,
    }
  }

  /**
   * 切换某进程的锁定态：指纹持久化到后端 cleaner_lock 表（与 PID 无关）。
   * 成功后本地同步列表状态；同指纹的其他实例由下次扫描自动标注。
   */
  async function toggleLock(p: DevProcInfo): Promise<void> {
    if (p.locked) {
      if (p.lock_id == null) return
      const lockId = p.lock_id
      const [, err] = await safeCall(() => removeCleanerLock(lockId))
      if (err) return
      for (const q of processes.value) {
        if (q.lock_id === lockId) {
          q.locked = false
          q.lock_id = null
        }
      }
      lockedCount.value = Math.max(0, lockedCount.value - 1)
    } else {
      const [lock, err] = await safeCall(() => addCleanerLock(toLockInput(p)))
      if (err || !lock) return
      for (const q of processes.value) {
        if (q.pid === p.pid) {
          q.locked = true
          q.lock_id = lock.id
        }
      }
      // 锁定时从选中集移除（锁定优先于选中）
      selectedPids.delete(p.pid)
      lockedCount.value += 1
    }
  }

  /** 解除全部锁定（清空 cleaner_lock 表所有条目，含未运行进程的）。
   *  返回是否成功（失败时本地锁定态保持不变，下轮扫描自动对齐）。 */
  async function unlockAll(): Promise<boolean> {
    if (lockedCount.value === 0) return true
    const [, err] = await safeCall(() => clearCleanerLocks())
    if (err) return false
    for (const q of processes.value) {
      q.locked = false
      q.lock_id = null
    }
    lockedCount.value = 0
    return true
  }

  /**
   * 切换「允许清理 IDE 进程」模式（危险模式）。
   * 重新开启 IDE 保护时，把已选中的 IDE PID 从选中集移除，并解除已锁定
   * IDE 进程的锁定（指纹条目从库中删除），避免状态不一致。
   */
  function toggleAllowKillIde() {
    allowKillIde.value = !allowKillIde.value
    if (!allowKillIde.value) {
      const unlockedIds = new Set<number>()
      for (const p of processes.value) {
        if (CATEGORY_META[p.category]?.protected !== true) continue
        selectedPids.delete(p.pid)
        if (p.locked && p.lock_id != null) {
          unlockedIds.add(p.lock_id)
          p.locked = false
          p.lock_id = null
        }
      }
      if (unlockedIds.size > 0) {
        // 后台批量删除锁定条目；先乐观扣减计数，完成后无论成败统一对账，
        // 避免个别删除失败导致 lockedCount 与库持久偏离
        lockedCount.value = Math.max(0, lockedCount.value - unlockedIds.size)
        void Promise.allSettled([...unlockedIds].map((id) => removeCleanerLock(id))).then(
          () => refreshLockCount(),
        )
      }
    }
  }

  /** 某进程是否可操作（非保护且非锁定）——选中、清理、回收的前置条件 */
  function isActionable(p: DevProcInfo): boolean {
    return !isProtected(p) && !isLocked(p)
  }

  /** 切换某进程的选中态（受保护或锁定的进程忽略） */
  function toggleSelect(pid: number) {
    const p = processes.value.find((x) => x.pid === pid)
    if (!p || !isActionable(p)) return
    if (selectedPids.has(pid)) selectedPids.delete(pid)
    else selectedPids.add(pid)
  }

  /** 选中/取消选中全部推荐项（跳过锁定的） */
  function selectAllRecommended() {
    // 仅推荐且未锁定的 PID 可参与全选
    const recs = recommendedProcs.value.filter((p) => !isLocked(p)).map((p) => p.pid)
    if (recs.length === 0) return
    // 若可参与的推荐项已全部选中 → 取消全部选中；否则全选
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
    if (!isActionable(p)) {
      return [null, isLocked(p) ? '该进程已锁定，不可清理' : '受保护的 IDE 进程不可清理']
    }
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

  // ===== 内存回收（修剪工作集，不杀进程） =====

  /** 公共 trim 执行器：设置 loading → 调用后端 → 刷新列表。 */
  async function doTrim(pids: number[]): Promise<[TrimResult | null, string | null]> {
    if (pids.length === 0) return [null, null]
    trimming.value = true
    try {
      const [result, error] = await safeCall(() => trimDevProcesses(pids))
      if (error || !result) return [null, error]
      void scan()
      return [result, null]
    } finally {
      trimming.value = false
    }
  }

  /** 回收所有已选中进程的内存（修剪工作集）。 */
  function trimSelected(): Promise<[TrimResult | null, string | null]> {
    const pids: number[] = []
    for (const p of processes.value) {
      if (selectedPids.has(p.pid)) pids.push(p.pid)
    }
    return doTrim(pids)
  }

  /** 回收单个进程的内存（行内「回收」按钮）。 */
  function trimOne(p: DevProcInfo): Promise<[TrimResult | null, string | null]> {
    if (isLocked(p)) return Promise.resolve([null, '该进程已锁定，不可回收'])
    return doTrim([p.pid])
  }

  /** 一键回收全部开发进程的内存（含 IDE，跳过锁定的）。 */
  function trimAll(): Promise<[TrimResult | null, string | null]> {
    const pids: number[] = []
    for (const p of processes.value) {
      if (!isLocked(p)) pids.push(p.pid)
    }
    return doTrim(pids)
  }

  return {
    processes,
    systemMem,
    scanning,
    killing,
    trimming,
    allowKillIde,
    selectedPids,
    recommendedPids,
    totalRecommendedMemory,
    totalProcessMemory,
    selectedMemory,
    selectedPidTrees,
    lockedCount,
    scan,
    startPolling,
    stopPolling,
    isProtected,
    isLocked,
    isActionable,
    toggleLock,
    unlockAll,
    toggleAllowKillIde,
    toggleSelect,
    selectAllRecommended,
    clearSelection,
    killSelected,
    killOne,
    trimSelected,
    trimOne,
    trimAll,
  }
})
