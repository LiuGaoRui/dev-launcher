// project store：项目列表 CRUD + 本地运行态跟踪。
//
// 运行态说明：
//   阶段 3 尚无「批量查询运行中项目」的后端命令（阶段 5 监控才会加），
//   这里用一个本地 Set 记忆「当前 session 内被前端启动且尚未停止」的 project_id。
//   初次加载时，若 DB 里 last_pid 不为空也不视为运行（重启 app 后 registry 必空），
//   以 registry（后端进程注册表）为准 —— 阶段 5 会用真实探测替换此简化逻辑。

import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  listProjects,
  createProject,
  updateProject,
  deleteProject,
} from '@/api/project'
import { startProject, stopProject, restartProject } from '@/api/process'
import type { Project, ProjectInput } from '@/types/project'
import { safeCall } from '@/api/invoke'

export const useProjectStore = defineStore('project', () => {
  const projects = ref<Project[]>([])
  const loading = ref(false)

  /** 本地记忆的「运行中」project_id 集合（阶段 5 替换为真实探测） */
  const runningIds = ref<Set<number>>(new Set())

  function setRunning(id: number, on: boolean) {
    if (on) runningIds.value.add(id)
    else runningIds.value.delete(id)
    runningIds.value = new Set(runningIds.value)
  }

  function updateProjectMeta(id: number, pid: number | null, startedAt: string | null) {
    const idx = projects.value.findIndex((x) => x.id === id)
    if (idx >= 0) {
      projects.value[idx].last_pid = pid
      if (startedAt) projects.value[idx].last_start_time = startedAt
    }
  }

  /** 拉取项目列表 */
  async function fetchAll(groupId?: number | null) {
    loading.value = true
    try {
      projects.value = await listProjects(groupId)
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

  /** 删除项目（运行中拒绝，见 ensureStopped） */
  async function remove(id: number) {
    await deleteProject(id)
    projects.value = projects.value.filter((p) => p.id !== id)
    runningIds.value.delete(id)
  }

  /** 启动项目：成功后记入 runningIds */
  async function start(id: number) {
    const r = await startProject(id)
    setRunning(id, true)
    updateProjectMeta(id, r.root_pid, r.started_at)
    return r
  }

  /** 停止项目：移出 runningIds */
  async function stop(id: number) {
    await stopProject(id)
    setRunning(id, false)
    updateProjectMeta(id, null, null)
  }

  /** 重启：stop → start */
  async function restart(id: number) {
    const r = await restartProject(id)
    setRunning(id, true)
    updateProjectMeta(id, r.root_pid, r.started_at)
    return r
  }

  function isRunning(id: number): boolean {
    return runningIds.value.has(id)
  }

  /** 安全执行任一 action，返回 [data, error] 二元组 */
  async function safe<T>(fn: () => Promise<T>): Promise<[T | null, string | null]> {
    return safeCall(fn)
  }

  /** 分组被删除时，清理本 store 中对应项目的 group_id 引用（DB 已由 ON DELETE SET NULL 兜底） */
  function onGroupDeleted(groupId: number) {
    for (const p of projects.value) {
      if (p.group_id === groupId) p.group_id = null
    }
  }

  return {
    projects,
    loading,
    runningIds,
    fetchAll,
    add,
    patch,
    remove,
    start,
    stop,
    restart,
    isRunning,
    safe,
    onGroupDeleted,
  }
})
