// 日志相关类型（对齐 Rust serde snake_case）

/** 日志类型：start=启动日志，build=构建日志 */
export type LogType = 'start' | 'build'

/** 实时日志推送的数据块 */
export interface LogChunk {
  /** 产生本块的订阅 id（后端分配），前端据此丢弃过期订阅的推送 */
  sub_id: number
  /** 本块数据在日志文件中的字节偏移 */
  offset: number
  /** 新增的文本内容（可能含多行） */
  text: string
}
