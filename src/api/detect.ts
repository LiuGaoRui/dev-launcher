// detect 命令薄包装。
// 对齐 src-tauri/src/commands/detect.rs：scan_projects。

import type { DetectedProject } from '@/types/detect'
import { invokeCmd } from './invoke'

/**
 * 扫描根目录下所有可识别的 Java / Node 项目。
 * @param root 绝对路径
 */
export function scanProjects(root: string): Promise<DetectedProject[]> {
  return invokeCmd<DetectedProject[]>('scan_projects', { root })
}
