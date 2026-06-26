// system 命令薄包装。
// 对齐 src-tauri/src/commands/system.rs：open_url。

import { invokeCmd } from './invoke'

/**
 * 用系统默认浏览器打开 URL。
 *
 * 后端仅放行 http/https 协议，拒绝其他（file://、UNC、可执行文件等）。
 */
export function openUrl(url: string): Promise<void> {
  return invokeCmd<void>('open_url', { url })
}
