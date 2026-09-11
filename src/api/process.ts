// process 命令薄包装。
// 对齐 src-tauri/src/commands/process.rs：
//   start_project / stop_project / build_project / stop_build / get_build_status。

import type { StartResult, BuildState } from '@/types/project'
import { invokeCmd } from './invoke'

/** 启动项目，返回 root pid / 日志路径 / 启动时间 */
export function startProject(id: number): Promise<StartResult> {
  return invokeCmd<StartResult>('start_project', { id })
}

/** 停止项目（Job terminate 杀整树 → 轮询归零 → 兜底 kill） */
export function stopProject(id: number): Promise<void> {
  return invokeCmd<void>('stop_project', { id })
}

/**
 * 触发后台构建：命令立即返回，构建在后台执行。
 * 前端轮询 getBuildStatus 获取构建状态（running/exit_code）。
 * 构建输出实时写入 build.log，由日志页订阅查看。
 */
export function buildProject(id: number): Promise<void> {
  return invokeCmd<void>('build_project', { id })
}

/** 停止构建：taskkill 杀构建进程树，状态置为已取消（canceled=true） */
export function stopBuild(id: number): Promise<void> {
  return invokeCmd<void>('stop_build', { id })
}

/** 查询某项目的构建状态（供轮询展示构建按钮状态图标） */
export function getBuildStatus(id: number): Promise<BuildState> {
  return invokeCmd<BuildState>('get_build_status', { id })
}
