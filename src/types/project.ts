// 与 src-tauri/src/models.rs 的 Project / ProjectInput / ProjectType 对齐。
// serde 默认输出 snake_case 字段名；ProjectType 为 snake_case 枚举字符串。

/**
 * 项目类型枚举。
 *
 * 值与 Rust `ProjectType` 的 `#[serde(rename_all = "snake_case")]` 一致，
 * 同时也是 DB `project.type` 列的 CHECK 约束值。
 */
export type ProjectType =
  | 'springboot'
  | 'java_jar'
  | 'node'
  | 'docker_compose'
  | 'custom'

/** 项目类型选项（表单下拉用） */
export const PROJECT_TYPE_OPTIONS: { label: string; value: ProjectType }[] = [
  { label: 'SpringBoot', value: 'springboot' },
  { label: 'Java Jar', value: 'java_jar' },
  { label: 'Node', value: 'node' },
  { label: 'Docker Compose', value: 'docker_compose' },
  { label: '自定义命令', value: 'custom' },
]

/** 类型 → 中文标签（卡片/状态展示用） */
export const PROJECT_TYPE_LABELS: Record<ProjectType, string> = {
  springboot: 'SpringBoot',
  java_jar: 'Java Jar',
  node: 'Node',
  docker_compose: 'Docker Compose',
  custom: '自定义',
}

/** 被管理的项目（对齐 Rust `Project`） */
export interface Project {
  id: number
  name: string
  /** Rust 字段为 `r#type`，serde 序列化为 `type` */
  type: ProjectType
  path: string
  /** 运行时工作目录（null 时用 path） */
  workdir: string | null
  /** 扫描根目录（扫描添加时记录，前端按此分面板展示；手动添加可空） */
  scan_root: string | null
  start_cmd: string
  build_cmd: string | null
  /** 端口字符串数组（DB 存为 JSON TEXT） */
  expected_ports: string[]
  enabled: boolean
  last_pid: number | null
  last_start_time: string | null
  last_stop_time: string | null
  create_time: string
  update_time: string
  /** 同一扫描目录面板内的排序值（list 时按 sort_order ASC, id ASC） */
  sort_order: number
}

/** 新建/更新项目入参（对齐 Rust `ProjectInput`，不含 id / 时间戳） */
export interface ProjectInput {
  name: string
  type: ProjectType
  path: string
  /** 运行时工作目录（可选，null 时用 path） */
  workdir: string | null
  /** 扫描根目录（可选） */
  scan_root: string | null
  start_cmd: string
  build_cmd: string | null
  expected_ports: string[]
  enabled: boolean
}

/** start_project / restart_project 的返回值（对齐 Rust `StartResult`） */
export interface StartResult {
  root_pid: number
  log_path: string
  started_at: string
}

/**
 * 构建输出事件（对齐 Rust `BuildEvent`，serde tag=kind/content=data）。
 *
 * kind=stdout/stderr 时 data 为文本；kind=exit 时 data 为退出码（数字）。
 * 前端用 discriminated union 收窄 data 类型。
 */
export type BuildEvent =
  | { kind: 'stdout'; data: string }
  | { kind: 'stderr'; data: string }
  | { kind: 'exit'; data: number }

/** build_project 的返回值（对齐 Rust `BuildResult`） */
export interface BuildResult {
  exit_code: number
  duration_ms: number
}
