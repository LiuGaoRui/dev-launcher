// project store：项目列表 CRUD + 运行态实时探测（阶段 5）。
//
// 运行态说明（阶段 5）：
//   通过 `probe_statuses` 命令每 3s 轮询后端 registry 中的运行项目，
//   返回的 statuses 是「项目 id → ProjectStatus」映射。不在映射中的项目视为 stopped。
//   启停操作成功后立即 probeNow() 刷新一次，避免等下一轮（乐观更新 + 真实探测兜底）。
//
// 轮询生命周期由视图层控制：ProjectList onMounted 调 startPolling()，
// onBeforeUnmount 调 stopPolling()。store 自身不感知组件生命周期。

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  listProjects,
  createProject,
  updateProject,
  deleteProject,
  reorderProjects,
} from '@/api/project'
import { startProject, stopProject, restartProject } from '@/api/process'
import { probeStatuses } from '@/api/monitor'
import { listScanRootOrder, reorderScanRoots } from '@/api/scan_root'
import type { Project, ProjectInput } from '@/types/project'
import type { HealthStatus, ProjectStatus } from '@/types/monitor'
import { safeCall } from '@/api/invoke'

/** 监控轮询间隔（ms），对齐开发计划 §5.4 的 3s 周期 */
const POLL_INTERVAL_MS = 3000

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const loading = ref(false)

  /** 运行中项目的探测结果：project_id → ProjectStatus。不在表中的 id 视为 stopped。 */
  const statuses = ref<Record<number, ProjectStatus>>({})

  /** 扫描目录（面板）的排序记录：scan_root → sort_order。未记录的目录前端按字母序兜底。 */
  const scanRootOrder = ref<Record<string, number>>({})

  /** 轮询定时器句柄（null 表示未在轮询） */
  let pollTimer: ReturnType<typeof setInterval> | null = null
  /** 防止并发 probe（上一轮未完成时跳过） */
  let probing = false

  function updateProjectMeta(id: number, pid: number | null, startedAt: string | null) {
    const idx = projects.value.findIndex((x) => x.id === id)
    if (idx >= 0) {
      projects.value[idx].last_pid = pid
      if (startedAt) projects.value[idx].last_start_time = startedAt
    }
  }

  /** 拉取项目列表 + 扫描目录排序 */
  async function fetchAll() {
    loading.value = true
    try {
      const [list, order] = await Promise.all([
        listProjects(),
        listScanRootOrder(),
      ])
      // 预 trim scan_root，避免 panels computed 每次重算都 trim
      for (const p of list) {
        if (p.scan_root) p.scan_root = p.scan_root.trim() || null
      }
      projects.value = list
      scanRootOrder.value = order
    } finally {
      loading.value = false
    }
  }

  /** 新建项目 */
  async function add(input: ProjectInput): Promise<Project> {
    const p = await createProject(input)
    projects.value.push(p)
    return p
  }

  /** 更新项目（全量覆盖） */
  async function patch(id: number, input: ProjectInput): Promise<Project> {
    const p = await updateProject(id, input)
    const idx = projects.value.findIndex((x) => x.id === id)
    if (idx >= 0) projects.value[idx] = p
    return p
  }

  /** 删除项目 */
  async function remove(id: number) {
    await deleteProject(id)
    projects.value = projects.value.filter((p) => p.id !== id)
    delete statuses.value[id]
  }

  /** 启动项目：乐观标记后立即探测确认 */
  async function start(id: number) {
    const r = await startProject(id)
    updateProjectMeta(id, r.root_pid, r.started_at)
    // 立即探测刷新状态（不等下一轮 3s）
    void probeNow()
    return r
  }

  /** 停止项目：移出 statuses 后立即探测确认 */
  async function stop(id: number) {
    await stopProject(id)
    delete statuses.value[id]
    updateProjectMeta(id, null, null)
    void probeNow()
  }

  /** 重启：stop → start */
  async function restart(id: number) {
    const r = await restartProject(id)
    updateProjectMeta(id, r.root_pid, r.started_at)
    void probeNow()
    return r
  }

  // ===== 运行态探测（阶段 5） =====

  /** 立即拉取一次运行态（重入安全：上一轮未完成则跳过） */
  async function probeNow() {
    if (probing) return
    probing = true
    try {
      const list = await probeStatuses()
      const map: Record<number, ProjectStatus> = {}
      for (const s of list) map[s.project_id] = s
      statuses.value = map
    } catch (e) {
      // 探测失败不应打断 UI；下次轮询重试
      console.warn('probe_statuses 失败:', e)
    } finally {
      probing = false
    }
  }

  /** 启动 3s 轮询（幂等：已运行则忽略） */
  function startPolling() {
    if (pollTimer !== null) return
    void probeNow()
    pollTimer = setInterval(() => void probeNow(), POLL_INTERVAL_MS)
  }

  /** 停止轮询 */
  function stopPolling() {
    if (pollTimer !== null) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  /** 取某项目的健康状态（无探测结果 → stopped） */
  function getHealth(id: number): HealthStatus {
    return statuses.value[id]?.health ?? 'stopped'
  }

  /** 是否运行中（running 或 running_abnormal） */
  function isRunning(id: number): boolean {
    const h = getHealth(id)
    return h === 'running' || h === 'running_abnormal'
  }

  /** 运行中的项目数 */
  const runningCount = computed(
    () => projects.value.filter((p) => isRunning(p.id)).length,
  )

  /** 项目总数 */
  const totalCount = computed(() => projects.value.length)

  /** 安全执行任一 action，返回 [data, error] 二元组 */
  async function safe<T>(fn: () => Promise<T>): Promise<[T | null, string | null]> {
    return safeCall(fn)
  }

  // ===== 排序（拖拽） =====

  /**
   * 重排项目顺序：先持久化到 DB，再本地重排 projects 数组。
   * @param ids 新顺序下的项目 id 数组（仅含被拖动面板内的项目）
   */
  async function reorderProjectsOrder(ids: number[]) {
    await reorderProjects(ids)
    // 本地重排：按 ids 顺序重排 projects 数组中对应项的位置
    const idIndex = new Map(ids.map((id, i) => [id, i]))
    const origIndex = new Map(projects.value.map((p, i) => [p.id, i]))
    projects.value.sort((a, b) => {
      const ai = idIndex.get(a.id)
      const bi = idIndex.get(b.id)
      if (ai !== undefined && bi !== undefined) return ai - bi
      if (ai !== undefined) return -1
      if (bi !== undefined) return 1
      return (origIndex.get(a.id) ?? 0) - (origIndex.get(b.id) ?? 0)
    })
  }

  /**
   * 重排扫描目录（面板）顺序：先持久化到 DB，再更新本地 scanRootOrder 缓存。
   * @param roots 新顺序下的扫描目录路径数组
   */
  async function reorderScanRootsOrder(roots: string[]) {
    await reorderScanRoots(roots)
    const map: Record<string, number> = {}
    roots.forEach((r, i) => (map[r] = i))
    scanRootOrder.value = map
  }

  return {
    projects,
    loading,
    statuses,
    scanRootOrder,
    fetchAll,
    add,
    patch,
    remove,
    start,
    stop,
    restart,
    probeNow,
    startPolling,
    stopPolling,
    getHealth,
    isRunning,
    runningCount,
    totalCount,
    safe,
    reorderProjectsOrder,
    reorderScanRootsOrder,
  }
})
