// 日志相关类型（对齐 Rust serde snake_case，见 docs/03-命令清单.md §六）

/** 实时日志推送的数据块 */
export interface LogChunk {
  /** 本块数据在日志文件中的字节偏移 */
  offset: number
  /** 新增的文本内容（可能含多行） */
  text: string
}

/** 历史日志分页读取结果 */
export interface LogHistoryPage {
  /** 日志文件总字节数 */
  total_size: number
  /** 本次读取到的文本（最多 limit 字节） */
  data: string
  /** 下一页起始偏移；null 表示已到文件末尾 */
  next_offset: number | null
}
