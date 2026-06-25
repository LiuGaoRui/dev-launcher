// group 命令薄包装。
// 对齐 src-tauri/src/commands/group.rs：list_groups / create_group / update_group / delete_group。

import type { Group, GroupInput, GroupUpdate } from '@/types/group'
import { invokeCmd } from './invoke'

/** 列出全部分组（按 order 升序） */
export function listGroups(): Promise<Group[]> {
  return invokeCmd<Group[]>('list_groups')
}

/** 新建分组 */
export function createGroup(input: GroupInput): Promise<Group> {
  return invokeCmd<Group>('create_group', { input })
}

/** 更新分组（name / order 可选） */
export function updateGroup(id: number, input: GroupUpdate): Promise<Group> {
  return invokeCmd<Group>('update_group', { id, input })
}

/** 删除分组（其下项目 group_id 置空） */
export function deleteGroup(id: number): Promise<void> {
  return invokeCmd<void>('delete_group', { id })
}
