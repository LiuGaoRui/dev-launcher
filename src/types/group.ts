// 与 src-tauri/src/models.rs 的 Group / GroupInput / GroupUpdate 对齐。
// serde 默认输出 snake_case 字段名（group.order / create_time）。

/** 项目分组（对齐 Rust `Group`） */
export interface Group {
  id: number
  name: string
  /** 排序值（升序）；Rust 字段名是 `order`（DB 关键字，sqlx rename） */
  order: number
  /** ISO 时间戳字符串 */
  create_time: string
}

/** 新建分组入参（对齐 Rust `GroupInput`） */
export interface GroupInput {
  name: string
}

/** 更新分组入参（对齐 Rust `GroupUpdate`），字段均可选 */
export interface GroupUpdate {
  name?: string
  order?: number
}
