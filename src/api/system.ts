// system 命令薄包装。
// 对齐 src-tauri/src/commands/system.rs：open_url / get_app_memory。

import { invokeCmd } from './invoke'

/**
 * 用系统默认浏览器打开 URL。
 *
 * 后端仅放行 http/https 协议，拒绝其他（file://、UNC 路径、可执行文件等）。
 */
export function openUrl(url: string): Promise<void> {
  return invokeCmd<void>('open_url', { url })
}

/**
 * 获取本应用自身进程的 RSS 内存（字节数）。
 *
 * 后端用 sysinfo 读取自身进程内存，供状态栏展示工具自身内存占用。
 */
export function getAppMemory(): Promise<number> {
  return invokeCmd<number>('get_app_memory')
}
