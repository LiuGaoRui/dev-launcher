// scan_root 命令薄包装。
// 对齐 src-tauri/src/commands/scan_root.rs：
// list_scan_root_order / reorder_scan_roots。

import { invokeCmd } from './invoke'

/** 读取全部已记录的扫描目录顺序：scan_root → sort_order */
export function listScanRootOrder(): Promise<Record<string, number>> {
  return invokeCmd<Record<string, number>>('list_scan_root_order')
}

/**
 * 按给定顺序批量重排扫描目录（面板拖拽排序后调用）。
 * @param roots 扫描目录路径数组，下标即新 sort_order
 */
export function reorderScanRoots(roots: string[]): Promise<void> {
  return invokeCmd<void>('reorder_scan_roots', { roots })
}
