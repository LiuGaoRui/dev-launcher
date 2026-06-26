// project 命令薄包装。
// 对齐 src-tauri/src/commands/project.rs：
// list_projects / get_project / create_project / update_project / delete_project。

import type { Project, ProjectInput } from '@/types/project'
import { invokeCmd } from './invoke'

/** 列出全部项目，按 id 升序 */
export function listProjects(): Promise<Project[]> {
  return invokeCmd<Project[]>('list_projects')
}

/** 按 id 取项目 */
export function getProject(id: number): Promise<Project> {
  return invokeCmd<Project>('get_project', { id })
}

/** 新建项目。input 字段须用 snake_case（对齐 Rust ProjectInput serde） */
export function createProject(input: ProjectInput): Promise<Project> {
  return invokeCmd<Project>('create_project', { input })
}

/** 更新项目（全量覆盖字段） */
export function updateProject(id: number, input: ProjectInput): Promise<Project> {
  return invokeCmd<Project>('update_project', { id, input })
}

/** 删除项目 */
export function deleteProject(id: number): Promise<void> {
  return invokeCmd<void>('delete_project', { id })
}

/**
 * 按给定顺序批量重排项目 sort_order（拖拽排序后调用）。
 * @param ids 项目 id 数组，下标即新 sort_order
 */
export function reorderProjects(ids: number[]): Promise<void> {
  return invokeCmd<void>('reorder_projects', { ids })
}
