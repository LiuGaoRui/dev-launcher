// 日志相关类型（对齐 Rust serde snake_case）

/** 日志类型：start=启动日志，build=构建日志 */
export type LogType = 'start' | 'build'

/** 实时日志推送的数据块 */
export interface LogChunk {
  /** 本块数据在日志文件中的字节偏移 */
  offset: number
  /** 新增的文本内容（可能含多行） */
  text: string
}
