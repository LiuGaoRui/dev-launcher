// group store：缓存分组列表，提供 CRUD actions。
//
// 分组数据量小且变化不频繁，列表页 + 分组管理页共享同一份缓存。
// 增删改后自动重载（或本地调整），保证 UI 即时一致。

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { listGroups, createGroup, updateGroup, deleteGroup } from '@/api/group'
import type { Group, GroupInput, GroupUpdate } from '@/types/group'
import { safeCall } from '@/api/invoke'

export const useGroupStore = defineStore('group', () => {
  const groups = ref<Group[]>([])
  const loading = ref(false)

  /** 拉取全部分组 */
  async function fetchAll() {
    loading.value = true
    try {
      groups.value = await listGroups()
    } finally {
      loading.value = false
    }
  }

  /** 新建分组 */
  async function add(input: GroupInput): Promise<Group> {
    const g = await createGroup(input)
    // 本地插入（按 order 升序保持顺序；新分组 order 最大，直接 push）
    groups.value.push(g)
    return g
  }

  /** 更新分组（name / order 可选） */
  async function patch(id: number, input: GroupUpdate): Promise<Group> {
    const g = await updateGroup(id, input)
    const idx = groups.value.findIndex((x) => x.id === id)
    if (idx >= 0) groups.value[idx] = g
    // 若改了 order 需重排
    if (input.order !== undefined) sortByOrder()
    return g
  }

  /** 删除分组：同时把项目 store 中该分组的项目 group_id 置 null（由后端 ON DELETE SET NULL 兜底） */
  async function remove(id: number) {
    await deleteGroup(id)
    groups.value = groups.value.filter((g) => g.id !== id)
  }

  async function safe<T>(fn: () => Promise<T>): Promise<[T | null, string | null]> {
    return safeCall(fn)
  }

  function sortByOrder() {
    groups.value.sort((a, b) => a.order - b.order)
  }

  return { groups, loading, fetchAll, add, patch, remove, safe }
})
