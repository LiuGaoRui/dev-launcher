// Tauri invoke 薄包装：统一错误解析 + 类型化。
//
// 关键约定（Tauri v2）：
// - 顶层命令参数名用 camelCase（Tauri 自动转 snake_case 匹配 Rust 参数）：groupId / id / input
// - 嵌套结构体（如 ProjectInput）的字段名须与 Rust serde 一致（snake_case），不做转换
//
// Rust 端 AppError 经 Serialize 序列化为字符串，IPC reject 后到达前端为 Error.message。

import { invoke, type InvokeArgs } from '@tauri-apps/api/core'

/**
 * 类型化 invoke 包装。
 *
 * Tauri reject 抛出的是 Error，message 为后端 AppError 的 to_string()。
 * 直接透传；调用方用 try/catch 或 .catch 处理并展示错误提示。
 */
export function invokeCmd<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  return invoke<T>(cmd, args)
}

/** 断言对象是 Error（Tauri reject 的异常） */
export function isError(e: unknown): e is Error {
  return e instanceof Error
}

/** 从未知异常中取 message（reject / throw 均适用） */
export function errMsg(e: unknown): string {
  if (isError(e)) return e.message
  if (typeof e === 'string') return e
  try {
    return JSON.stringify(e)
  } catch {
    return String(e)
  }
}

/**
 * 安全执行异步函数，返回 [data, error] 二元组。
 * store / 视图层共用，避免在每个 store 中重复 try/catch。
 */
export async function safeCall<T>(fn: () => Promise<T>): Promise<[T | null, string | null]> {
  try {
    const data = await fn()
    return [data, null]
  } catch (e) {
    return [null, errMsg(e)]
  }
}
