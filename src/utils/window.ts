// 窗口控制封装 —— 集成无边框标题栏所需的最小化/最大化/关闭。
//
// Tauri 2 的窗口 API：getCurrentWindow() 返回当前 webview 绑定的窗口句柄。
// core:default 已含 minimize / toggleMaximize / close 权限，无需额外 capability。

import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()

/** 最小化窗口 */
export function minimizeWindow(): Promise<void> {
  return appWindow.minimize()
}

/** 切换最大化 / 还原 */
export function toggleMaximize(): Promise<void> {
  return appWindow.toggleMaximize()
}

/** 关闭窗口 */
export function closeWindow(): Promise<void> {
  return appWindow.close()
}

/** 查询当前是否最大化（标题栏按钮图标切换用） */
export function isMaximized(): Promise<boolean> {
  return appWindow.isMaximized()
}

/**
 * 监听最大化状态变化。
 * @returns 取消监听的清理函数
 */
export function onMaximizedChange(cb: (maximized: boolean) => void): Promise<() => void> {
  return appWindow
    .onResized(async () => {
      cb(await appWindow.isMaximized())
    })
    .then((unlisten) => unlisten)
}
