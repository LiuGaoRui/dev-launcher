// project 命令薄包装。
// 对齐 src-tauri/src/commands/project.rs：
// list_projects / get_project / create_project / update_project / delete_project。

import type { Project, ProjectInput } from '@/types/project'
import { invokeCmd } from './invoke'

/**
 * 列出项目。
 * @param groupId 省略/为 null 列全部；传值则只列该分组
 */
export function listProjects(groupId?: number | null): Promise<Project[]> {
  return invokeCmd<Project[]>('list_projects', { groupId: groupId ?? null })
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
