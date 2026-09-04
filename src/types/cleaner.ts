// 与 src-tauri/src/process/dev_scan.rs 的 DevProcCategory / DevProcInfo 对齐。
// 与 src-tauri/src/commands/cleaner.rs 的 KillResult / SystemMemory 对齐。
// 与 src-tauri/src/models.rs 的 CleanerLock / CleanerLockInput 对齐。
// serde 默认输出 snake_case；DevProcCategory 为 snake_case 枚举字符串。

/**
 * 开发进程智能分类标签（对齐 Rust `DevProcCategory`）。
 *
 * - `ide_main`：IDE 自身进程（IDEA/VSCode），受保护不可清理
 * - `build_daemon`：构建 daemon（Gradle/Maven daemon、编译器常驻进程）
 * - `dev_server`：dev server（Vite/webpack/SpringBoot dev 模式）
 * - `orphan`：孤立进程（父祖链无 IDE，通常是关闭项目后未释放的残留）
 * - `watcher`：文件监视器（tsc/esbuild --watch、nodemon）
 * - `other`：其他 java/node 开发进程
 */
export type DevProcCategory =
  | 'ide_main'
  | 'build_daemon'
  | 'dev_server'
  | 'orphan'
  | 'watcher'
  | 'other'

/** 分类 → 中文标签 + 颜色语义（供 UI 标签渲染） */
export const CATEGORY_META: Record<
  DevProcCategory,
  { label: string; color: string; protected?: boolean }
> = {
  ide_main: { label: 'IDE 进程', color: '#8c8c8c', protected: true },
  build_daemon: { label: '构建守护', color: '#722ed1' },
  dev_server: { label: 'Dev Server', color: '#1890ff' },
  orphan: { label: '孤立进程', color: '#faad14' },
  watcher: { label: '文件监视', color: '#13c2c2' },
  other: { label: '其他', color: '#8c8c8c' },
}

/** 单个开发进程的扫描结果（对齐 Rust `DevProcInfo`） */
export interface DevProcInfo {
  pid: number
  /** 父进程 PID */
  ppid: number | null
  /** 进程大类：java | node */
  kind: string
  /** 进程名（如 java.exe、node.exe） */
  name: string
  /** 可执行文件路径 */
  exe: string
  /** 完整命令行（已截断） */
  cmdline: string
  /** 从命令行提取的项目/模块提示 */
  cmdline_hint: string
  /** 用户友好的展示名（如「IntelliJ IDEA」「VSCode 扩展宿主」「vite dev server」） */
  display_title: string
  /** 关联的项目/工作目录路径（帮助判断是哪个项目的进程），可能为空 */
  project_path: string
  /** 进程工作目录（OS 报告的真实 cwd），可能为 null（权限不足或已退出） */
  cwd: string | null
  /** 命令行语义化摘要：提取主脚本 + 关键参数，比 exe 路径更能说明「在做什么」 */
  cmdline_summary: string
  /** 正在执行的主程序名（如 server.js / Application / demo.jar），用于快速辨识 */
  main_script: string
  /** 本进程 RSS（字节） */
  memory_bytes: number
  /** 整个进程树内存（含子进程，字节） */
  tree_memory_bytes: number
  /** 整树 CPU 占用百分比（归一化到 0-100） */
  cpu_percent: number
  /** 启动时间（ISO 字符串） */
  started_at: string | null
  /** 智能分类 */
  category: DevProcCategory
  /** 整树 PID 列表（清理时原样回传后端） */
  tree_pid_list: number[]
  /** 是否推荐清理 */
  recommended: boolean
  /** 规范化进程指纹（后端基于未截断完整命令行计算，锁定匹配键）。
   *  锁定时原样回传后端，前端不解析。 */
  fingerprint: string
  /** 是否被用户手动锁定（后端按进程指纹匹配 cleaner_lock 表，与 PID 无关） */
  locked: boolean
  /** 匹配到的锁定条目 id（未锁定为 null；解锁时回传后端） */
  lock_id: number | null
}

/** 批量杀进程的结果（对齐 Rust `KillResult`） */
export interface KillResult {
  /** 成功杀死的进程树数量 */
  killed: number
  /** 杀失败的进程树数量 */
  failed: number
  /** 因命中锁定表被跳过的进程树数量（后端纵深防御） */
  skipped_locked: number
  /** 杀死前这些进程树的内存总和（字节） */
  freed_bytes: number
}

/** 批量修剪工作集（内存回收，不杀进程）的结果（对齐 Rust `TrimResult`） */
export interface TrimResult {
  /** 成功修剪的进程数量 */
  trimmed: number
  /** 修剪失败的进程数量（权限不足或进程已退出） */
  failed: number
  /** 因命中锁定表被跳过的进程数量（后端纵深防御） */
  skipped_locked: number
  /** 回收的物理内存总量（字节，回收前后 RSS 差值之和） */
  freed_bytes: number
}

/** 系统内存概况（对齐 Rust `SystemMemory`） */
export interface SystemMemory {
  /** 物理内存总量（字节） */
  total_bytes: number
  /** 已用（字节） */
  used_bytes: number
  /** 可用 = total - used */
  available_bytes: number
  /** 使用率（0-100） */
  used_percent: number
}

/**
 * 手动锁定的进程指纹记录（对齐 Rust `CleanerLock`，cleaner_lock 表）。
 *
 * 匹配键是 fingerprint（后端由 cmdline/cwd/name 规范化生成），与 PID 无关——
 * 进程重启后 PID 变化不影响锁定。其余字段为锁定时的原始指纹，仅供展示。
 */
export interface CleanerLock {
  id: number
  /** 规范化匹配键 */
  fingerprint: string
  /** 进程名（如 java.exe） */
  name: string
  /** 完整命令行（锁定时） */
  cmdline: string
  /** 启动目录（锁定时），可能为 null */
  cwd: string | null
  /** 可执行文件路径 */
  exe: string
  /** 展示名（如「vite dev」） */
  display_title: string
  /** 命令行摘要 */
  cmdline_summary: string
  /** 锁定时的 PID（仅参考，不参与匹配） */
  locked_pid: number
  /** 锁定时间 */
  create_time: string
}

/** 锁定一个进程的输入（对齐 Rust `CleanerLockInput`，字段取自 DevProcInfo） */
export interface CleanerLockInput {
  /** 规范化匹配键（DevProcInfo.fingerprint 原样回传，后端不重算） */
  fingerprint: string
  name: string
  cmdline: string
  cwd: string | null
  exe: string
  display_title: string
  cmdline_summary: string
  /** 锁定时的 PID（仅参考） */
  pid: number
}
