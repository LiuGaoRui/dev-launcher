// port store：本机监听端口扫描 + 端口监控器状态。
//
// 数据由后端 list_listening_ports 一次性分类评分排序返回，本 store 不做二次排序，
// 只提供分组视图（供弹窗分区渲染）与可疑计数（供 TitleBar 角标）。
//
// 轮询生命周期分散在两处（都只调 scan()，靠 scanning 防重入共存）：
// - TitleBar：常驻低频（30s）轮询驱动红色角标
// - PortMonitorModal：打开期间高频（5s）轮询
//
// 杀进程复用 cleaner 域的 kill_dev_processes（taskkill /F /T 杀整树，
// 含锁定表/自身护栏），本 store 不重复封装。
//
// 忽略名单（port_ignore 表，按端口号持久化）：后端在扫描时统一覆盖为
// ignored 分类，因此忽略/恢复后只需重新扫描即可看到分组与角标变化。

import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { listListeningPorts, addPortIgnore, removePortIgnore } from '@/api/port'
import { killDevProcesses } from '@/api/cleaner'
import type { ListeningPortInfo, PortCategory } from '@/types/port'
import type { KillResult } from '@/types/cleaner'
import { safeCall } from '@/api/invoke'

/** 可疑端口数角标封顶值（>99 显示 99+） */
export const BADGE_MAX = 99

export const usePortStore = defineStore('port', () => {
  /** 扫描到的监听端口列表（后端已按 score 降序） */
  const ports = ref<ListeningPortInfo[]>([])
  /** 是否正在扫描中（防止并发轮询重入） */
  const scanning = ref(false)
  /** 是否正在执行结束进程 */
  const killing = ref(false)

  /** 立即扫描一次（重入安全：上一轮未完成则跳过） */
  async function scan() {
    if (scanning.value) return
    scanning.value = true
    try {
      const [data, err] = await safeCall(() => listListeningPorts())
      if (!err && data) ports.value = data
      // 扫描失败不打断 UI；下次轮询重试
      else if (err) console.warn('list_listening_ports 失败:', err)
    } finally {
      scanning.value = false
    }
  }

  // ===== 计算 =====

  /** 可疑端口数（TitleBar 红色角标） */
  const suspiciousCount = computed(
    () => ports.value.filter((p) => p.category === 'suspicious').length,
  )

  /** 各分类计数（弹窗顶部统计条） */
  const categoryCounts = computed(() => {
    const counts = {} as Record<PortCategory, number>
    for (const p of ports.value) {
      counts[p.category] = (counts[p.category] ?? 0) + 1
    }
    return counts
  })

  /** 重点区：可疑 + 开发 + 其他（弹窗平铺区，保持后端 score 排序） */
  const focusPorts = computed(() =>
    ports.value.filter((p) =>
      ['suspicious', 'dev', 'other'].includes(p.category),
    ),
  )

  /** 已配置项目端口（托管 + 外部启动合并展示） */
  const knownProjectPorts = computed(() =>
    ports.value.filter(
      (p) => p.category === 'known_project' || p.category === 'known_project_external',
    ),
  )

  /** 环境服务端口 */
  const envServicePorts = computed(
    () => ports.value.filter((p) => p.category === 'env_service'),
  )

  /** 系统端口 */
  const systemPorts = computed(
    () => ports.value.filter((p) => p.category === 'system'),
  )

  /** 已忽略端口（port_ignore 名单命中，仍监听中，可恢复） */
  const ignoredPorts = computed(
    () => ports.value.filter((p) => p.category === 'ignored'),
  )

  // ===== 操作 =====

  /**
   * 结束某端口占用者的整棵进程树（复用 kill_dev_processes 的护栏）。
   * 返回 [result, error]；成功后立即重新扫描。
   */
  async function killPort(p: ListeningPortInfo): Promise<[KillResult | null, string | null]> {
    killing.value = true
    try {
      const [result, error] = await safeCall(() => killDevProcesses([p.tree_pid_list]))
      if (error || !result) return [null, error]
      void scan()
      return [result, null]
    } finally {
      killing.value = false
    }
  }

  /**
   * 忽略一个端口（可疑行「忽略」按钮）。按端口号持久化到后端 port_ignore 表，
   * 成功后重新扫描——该端口移入「已忽略」折叠区且不再计入角标。
   */
  async function ignorePort(p: ListeningPortInfo): Promise<boolean> {
    const [, err] = await safeCall(() => addPortIgnore(p.port))
    if (err) return false
    void scan()
    return true
  }

  /**
   * 恢复一个被忽略的端口（「已忽略」折叠区「恢复」按钮）。
   * 成功后重新扫描——该端口回到正常分类（若仍可疑则重新计入角标）。
   */
  async function unignorePort(p: ListeningPortInfo): Promise<boolean> {
    const [, err] = await safeCall(() => removePortIgnore(p.port))
    if (err) return false
    void scan()
    return true
  }

  return {
    ports,
    scanning,
    killing,
    suspiciousCount,
    categoryCounts,
    focusPorts,
    knownProjectPorts,
    envServicePorts,
    systemPorts,
    ignoredPorts,
    scan,
    killPort,
    ignorePort,
    unignorePort,
  }
})
