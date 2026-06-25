<script setup lang="ts">
// 分组管理页。
//
// 能力：新建分组（顶部）、表格内重命名、上移/下移改 order、删除。
// 删除分组后，其下项目 group_id 由 DB ON DELETE SET NULL 置空；
// 这里同步通知 project store 清理本地引用。

import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { useGroupStore } from '@/stores/group'
import { useProjectStore } from '@/stores/project'
import type { Group } from '@/types/group'

const groupStore = useGroupStore()
const projectStore = useProjectStore()

onMounted(() => groupStore.fetchAll())

/** 分组 → 项目数映射（预计算，模板中 O(1) 查表） */
const groupProjectCounts = computed(() => {
  const map = new Map<number, number>()
  for (const p of projectStore.projects) {
    if (p.group_id != null) {
      map.set(p.group_id, (map.get(p.group_id) ?? 0) + 1)
    }
  }
  return map
})

function projectCountOf(groupId: number): number {
  return groupProjectCounts.value.get(groupId) ?? 0
}

/** 表格数据按 order 升序（store 已保证有序，直接引用） */
const tableData = computed(() => groupStore.groups)

// ===== 新建 =====

const createForm = reactive({ name: '' })
const createFormRef = ref<FormInstance>()
const creating = ref(false)

const createRules: FormRules<{ name: string }> = {
  name: [{ required: true, message: '请输入分组名称', trigger: 'blur' }],
}

async function handleCreate() {
  if (!createFormRef.value) return
  const valid = await createFormRef.value.validate().catch(() => false)
  if (!valid) return
  creating.value = true
  const [, err] = await groupStore.safe(() =>
    groupStore.add({ name: createForm.name.trim() }),
  )
  creating.value = false
  if (err) {
    ElMessage.error(`创建失败：${err}`)
    return
  }
  ElMessage.success('分组已创建')
  createForm.name = ''
  createFormRef.value.clearValidate()
}

// ===== 重命名（行内编辑） =====

const editingId = ref<number | null>(null)
const editingName = ref('')

function startRename(row: Group) {
  editingId.value = row.id
  editingName.value = row.name
}

async function saveRename(row: Group) {
  const name = editingName.value.trim()
  editingId.value = null
  if (!name || name === row.name) return
  const [, err] = await groupStore.safe(() =>
    groupStore.patch(row.id, { name }),
  )
  if (err) ElMessage.error(`重命名失败：${err}`)
  else ElMessage.success('已更新')
}

// ===== 排序（上移/下移） =====

const sorting = ref(false)

async function move(row: Group, dir: -1 | 1) {
  const sorted = tableData.value
  const idx = sorted.findIndex((g) => g.id === row.id)
  const target = idx + dir
  if (target < 0 || target >= sorted.length) return
  const other = sorted[target]
  sorting.value = true
  // 交换两者的 order
  const [, err] = await groupStore.safe(async () => {
    await groupStore.patch(row.id, { order: other.order })
    await groupStore.patch(other.id, { order: row.order })
  })
  sorting.value = false
  if (err) ElMessage.error(`移动失败：${err}`)
}

// ===== 删除 =====

async function handleDelete(row: Group) {
  try {
    await ElMessageBox.confirm(
      `确定删除分组「${row.name}」吗？该分组下的项目将变为「未分组」（不会被删除）。`,
      '删除确认',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  const [, err] = await groupStore.safe(() => groupStore.remove(row.id))
  if (err) {
    ElMessage.error(`删除失败：${err}`)
    return
  }
  // 同步清理 project store 中该分组的引用
  projectStore.onGroupDeleted(row.id)
  ElMessage.success('已删除')
}
</script>

<template>
  <div class="group-manage-page">
    <!-- 顶部：新建分组 -->
    <el-card shadow="never" class="create-card">
      <el-form
        ref="createFormRef"
        :model="createForm"
        :rules="createRules"
        inline
        @submit.prevent
      >
        <el-form-item label="新建分组" prop="name">
          <el-input
            v-model="createForm.name"
            placeholder="如：HR系统 / 中间件 / 测试项目"
            style="width: 260px"
            clearable
            @keyup.enter="handleCreate"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="creating" @click="handleCreate">
            创建
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 分组列表 -->
    <el-card shadow="never" class="table-card">
      <template #header>
        <span>分组列表</span>
      </template>
      <el-table :data="tableData" v-loading="groupStore.loading" row-key="id">
        <el-table-column label="排序" width="80" align="center">
          <template #default="{ row, $index }">
            <el-button-group>
              <el-button
                size="small"
                :disabled="$index === 0 || sorting"
                @click="move(row, -1)"
              >
                ↑
              </el-button>
              <el-button
                size="small"
                :disabled="$index === tableData.length - 1 || sorting"
                @click="move(row, 1)"
              >
                ↓
              </el-button>
            </el-button-group>
          </template>
        </el-table-column>

        <el-table-column label="分组名称" min-width="200">
          <template #default="{ row }">
            <template v-if="editingId === row.id">
              <el-input
                v-model="editingName"
                size="small"
                style="width: 200px"
                @keyup.enter="saveRename(row)"
                @blur="saveRename(row)"
              />
            </template>
            <template v-else>
              <span class="group-name" @dblclick="startRename(row)">{{ row.name }}</span>
            </template>
          </template>
        </el-table-column>

        <el-table-column label="项目数" width="100" align="center">
          <template #default="{ row }">
            <el-tag size="small" type="info" effect="plain">
              {{ projectCountOf(row.id) }}
            </el-tag>
          </template>
        </el-table-column>

        <el-table-column label="排序值" width="90" align="center" prop="order" />

        <el-table-column label="创建时间" width="180" prop="create_time" />

        <el-table-column label="操作" width="180" align="center">
          <template #default="{ row }">
            <el-button size="small" text @click="startRename(row)">重命名</el-button>
            <el-button
              size="small"
              type="danger"
              text
              @click="handleDelete(row)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-empty
        v-if="!groupStore.loading && tableData.length === 0"
        description="还没有分组"
      />
    </el-card>
  </div>
</template>

<style scoped>
.group-manage-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.create-card :deep(.el-card__body) {
  padding: 14px 16px 0;
}

.table-card {
  min-height: 300px;
}

.group-name {
  cursor: text;
  padding: 2px 4px;
  border-radius: 3px;
}
.group-name:hover {
  background: #f5f7fa;
}
</style>
