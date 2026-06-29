// 与 src-tauri/src/process/monitor.rs 的 HealthStatus / PortStatus / ProjectStatus 对齐。
// serde 默认输出 snake_case；HealthStatus 为 snake_case 枚举字符串。

/**
 * 项目健康状态（对齐开发计划 §5.2 状态机，简化三态）。
 *
 * - `stopped`：未运行
 * - `running`：进程存活，且（无预期端口 或 全部端口 listening && owned）
 * - `running_abnormal`：进程存活，但存在预期端口未监听 / 被非本项目占用
 */
export type HealthStatus = 'stopped' | 'running' | 'running_abnormal'

/** 状态 → 中文标签（StatusBadge / 详情用） */
export const HEALTH_LABELS: Record<HealthStatus, string> = {
  stopped: '已停止',
  running: '运行中',
  running_abnormal: '运行中(异常)',
}

/** 单个预期端口的探测结果（对齐 Rust `PortStatus`） */
export interface PortStatus {
  /** 端口号字符串（与 project.expected_ports 一致） */
  port: string
  /** 是否处于 TCP LISTEN */
  listening: boolean
  /** 占用者 PID 是否属于本项目进程树（仅在 listening 时有意义） */
  owned: boolean
}

/** 单个项目的完整探测结果（对齐 Rust `ProjectStatus`） */
export interface ProjectStatus {
  project_id: number
  health: HealthStatus
  /** 根进程 PID（运行中时来自 registry snapshot） */
  pid: number | null
  /** 整树 CPU 占用百分比（已归一化到 0-100） */
  cpu_percent: number
  /** 整树内存（RSS）字节数 */
  memory_bytes: number
  /** 各预期端口的探测结果 */
  ports: PortStatus[]
  /** 启动时间 */
  started_at: string | null
}

/**
 * 把字节数格式化为人类可读（如 1.2 GB）。
 * 用 1024 进制（与 RSS 物理含义一致）。
 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let v = bytes
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  // < 10 显示 1 位小数，否则取整
  const digits = v < 10 ? 1 : 0
  return `${v.toFixed(digits)} ${units[i]}`
}
