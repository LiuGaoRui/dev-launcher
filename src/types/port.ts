// 与 src-tauri/src/process/port_scan.rs 的 PortCategory / ListeningPortInfo 对齐。
// serde 默认输出 snake_case；PortCategory 为 snake_case 枚举字符串。

/**
 * 端口关注度分类（对齐 Rust `PortCategory`）。
 *
 * score 区间：suspicious 70+ / dev 50 / other 30 /
 * known_project_external 20 / known_project 15 / env_service 10 / system 0 / ignored 0
 */
export type PortCategory =
  | 'suspicious'
  | 'dev'
  | 'other'
  | 'known_project_external'
  | 'known_project'
  | 'env_service'
  | 'system'
  | 'ignored'

/** 分类 → 中文标签 + 颜色语义（供 UI 标签渲染） */
export const PORT_CATEGORY_META: Record<
  PortCategory,
  { label: string; color: string }
> = {
  suspicious: { label: '可疑', color: '#f5222d' },
  dev: { label: '开发端口', color: '#1890ff' },
  other: { label: '其他', color: '#8c8c8c' },
  known_project_external: { label: '已配置·外部启动', color: '#722ed1' },
  known_project: { label: '已配置项目', color: '#52c41a' },
  env_service: { label: '环境服务', color: '#13c2c2' },
  system: { label: '系统', color: '#d9d9d9' },
  ignored: { label: '已忽略', color: '#8c8c8c' },
}

/** 单个监听端口的扫描结果（对齐 Rust `ListeningPortInfo`） */
export interface ListeningPortInfo {
  /** 监听端口号 */
  port: number
  /** 监听地址列表（去重合并 v4/v6，如 "0.0.0.0:8080" / "[::]:8080"） */
  addresses: string[]
  /** 是否监听所有网卡（0.0.0.0 / [::]，对外暴露） */
  listen_all_interfaces: boolean
  /** 占用者 PID（内核监听等场景为 null） */
  owner_pid: number | null
  /** 进程名（如 node.exe） */
  process_name: string
  /** 可执行文件路径 */
  exe: string
  /** 用户友好的展示名（如「vite dev server」「server.js (node)」） */
  display_title: string
  /** 命令行语义化摘要 */
  cmdline_summary: string
  /** 关联的项目/工作目录路径，可能为空 */
  project_path: string
  /** 进程启动时间（ISO 字符串，不可得为 null） */
  started_at: string | null
  /** 占用者 RSS（字节） */
  memory_bytes: number
  /** 关注度分类 */
  category: PortCategory
  /** 关注度分数（0-100，后端已按降序排序；ignored 为 0 沉底） */
  score: number
  /** 父祖链无 IDE（AI CLI / 终端启动的典型特征） */
  orphan: boolean
  /** 是否为随机高位端口（非知名 dev 端口，测试框架常用） */
  random_port: boolean
  /** 命中的本工具项目名（端口命中 expected_ports 时） */
  matched_project_name: string | null
  /** 命中项目是否本工具托管运行中（false = 外部/AI 启动） */
  matched_project_managed: boolean
  /** 整树 PID 列表（「结束进程」时原样回传 kill_dev_processes） */
  tree_pid_list: number[]
}

/** ISO 时间 → 相对时间描述（如「3 分钟前」「2 小时前」「3 天前」），未知返回 '' */
export function formatRelativeTime(iso: string | null): string {
  if (!iso) return ''
  const t = new Date(iso.replace(' ', 'T')).getTime()
  if (Number.isNaN(t)) return ''
  const diffSec = Math.max(0, Math.floor((Date.now() - t) / 1000))
  if (diffSec < 60) return '刚刚'
  if (diffSec < 3600) return `${Math.floor(diffSec / 60)} 分钟前`
  if (diffSec < 86400) return `${Math.floor(diffSec / 3600)} 小时前`
  return `${Math.floor(diffSec / 86400)} 天前`
}
