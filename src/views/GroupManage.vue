<script setup lang="ts">
// 分组管理页。
//
// 能力：新建分组（顶部）、表格内重命名、上移/下移改 order、删除。
// 删除分组后，其下项目 group_id 由 DB ON DELETE SET NULL 置空；
// 这里同步通知 project store 清理本地引用。

import { computed, h, onMounted, reactive, ref } from 'vue'
import {
  NButton,
  NTag,
  NSpace,
  NInput,
  NForm,
  NFormItem,
  NDataTable,
  NEmpty,
  useMessage,
  useDialog,
  type DataTableColumns,
} from 'naive-ui'
import {
  ChevronUpOutline,
  ChevronDownOutline,
} from '@vicons/ionicons5'
import { useGroupStore } from '@/stores/group'
import { useProjectStore } from '@/stores/project'
import type { Group } from '@/types/group'

const groupStore = useGroupStore()
const projectStore = useProjectStore()
const message = useMessage()
const dialog = useDialog()

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

// ===== 行内重命名状态 =====
const editingId = ref<number | null>(null)
const editingName = ref('')

function startRename(row: Group) {
  editingId.value = row.id
  editingName.value = row.name
}

async function saveRename(row: Group) {
  if (editingId.value !== row.id) return
  const name = editingName.value.trim()
  editingId.value = null
  if (!name || name === row.name) return
  const [, err] = await groupStore.safe(() =>
    groupStore.patch(row.id, { name }),
  )
  if (err) message.error(`重命名失败：${err}`)
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
  const [, err] = await groupStore.safe(async () => {
    await groupStore.patch(row.id, { order: other.order })
    await groupStore.patch(other.id, { order: row.order })
  })
  sorting.value = false
  if (err) message.error(`移动失败：${err}`)
}

// ===== 删除 =====
async function handleDelete(row: Group) {
  dialog.warning({
    title: '删除确认',
    content: `确定删除分组「${row.name}」吗？该分组下的项目将变为「未分组」（不会被删除）。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      const [, err] = await groupStore.safe(() => groupStore.remove(row.id))
      if (err) {
        message.error(`删除失败：${err}`)
        return
      }
      projectStore.onGroupDeleted(row.id)
    },
  })
}

// ===== n-data-table 列定义 =====
const columns = computed<DataTableColumns<Group>>(() => [
  {
    title: '排序',
    key: 'sort',
    width: 90,
    align: 'center',
    render: (_row, index) =>
      h(NSpace, { justify: 'center', size: 'small', wrapItem: false }, () => [
        h(
          NButton,
          {
            size: 'tiny',
            quaternary: true,
            disabled: index === 0 || sorting.value,
            onClick: () => move(tableData.value[index], -1),
          },
          { icon: () => h(ChevronUpOutline) },
        ),
        h(
          NButton,
          {
            size: 'tiny',
            quaternary: true,
            disabled: index === tableData.value.length - 1 || sorting.value,
            onClick: () => move(tableData.value[index], 1),
          },
          { icon: () => h(ChevronDownOutline) },
        ),
      ]),
  },
  {
    title: '分组名称',
    key: 'name',
    minWidth: 200,
    render: (row) =>
      editingId.value === row.id
        ? h(NInput, {
            // 非受控：用 defaultValue 初始化，避免 columns computed 不追踪
            // editingName 导致输入时光标跳动。输入由组件内部管理，
            // saveRename 时从 editingName 取最新值即可。
            defaultValue: editingName.value,
            size: 'small',
            style: 'width: 200px',
            autofocus: true,
            'onUpdate:value': (v: string) => (editingName.value = v),
            onKeyup: (e: KeyboardEvent) => {
              if (e.key === 'Enter') saveRename(row)
            },
            onBlur: () => saveRename(row),
          })
        : h(
            'span',
            {
              class: 'group-name',
              onDblclick: () => startRename(row),
            },
            row.name,
          ),
  },
  {
    title: '项目数',
    key: 'count',
    width: 100,
    align: 'center',
    render: (row) =>
      h(
        NTag,
        { size: 'small', bordered: false, type: 'info' },
        () => String(projectCountOf(row.id)),
      ),
  },
  { title: '排序值', key: 'order', width: 90, align: 'center' },
  { title: '创建时间', key: 'create_time', width: 180 },
  {
    title: '操作',
    key: 'actions',
    width: 160,
    align: 'center',
    render: (row) =>
      h(NSpace, { justify: 'center', size: 'small' }, () => [
        h(
          NButton,
          { size: 'small', quaternary: true, onClick: () => startRename(row) },
          () => '重命名',
        ),
        h(
          NButton,
          {
            size: 'small',
            quaternary: true,
            type: 'error',
            onClick: () => handleDelete(row),
          },
          () => '删除',
        ),
      ]),
  },
])

// ===== 新建分组表单 =====
const createFormRef = ref()
const createForm = reactive({ name: '' })
const creating = ref(false)
const createRules = {
  name: { required: true, message: '请输入分组名称', trigger: 'blur' },
}

async function handleCreate() {
  createFormRef.value?.validate(async (errors: unknown) => {
    if (errors) return
    creating.value = true
    const [, err] = await groupStore.safe(() =>
      groupStore.add({ name: createForm.name.trim() }),
    )
    creating.value = false
    if (err) {
      message.error(`创建失败：${err}`)
      return
    }
    createForm.name = ''
    createFormRef.value?.restoreValidation()
  })
}
</script>

<template>
  <div class="group-manage-page">
    <div class="page-head">
      <span class="page-name">分组管理</span>
    </div>

    <!-- 新建分组 -->
    <div class="create-card card-surface">
      <NForm
        ref="createFormRef"
        :model="createForm"
        :rules="createRules"
        inline
        label-placement="left"
        @submit.prevent
      >
        <NFormItem label="新建分组" path="name">
          <NInput
            v-model:value="createForm.name"
            placeholder="如：HR系统 / 中间件 / 测试项目"
            style="width: 260px"
            clearable
            @keyup.enter="handleCreate"
          />
        </NFormItem>
        <NFormItem :show-label="false">
          <NButton type="primary" :loading="creating" @click="handleCreate">
            创建
          </NButton>
        </NFormItem>
      </NForm>
    </div>

    <!-- 分组列表 -->
    <div class="table-card card-surface">
      <div class="card-title">分组列表</div>
      <NDataTable
        :columns="columns"
        :data="tableData"
        :loading="groupStore.loading"
        :row-key="(row: Group) => row.id"
        :bordered="false"
        size="small"
        flex-height
        style="height: calc(100% - 44px)"
      />
      <NEmpty
        v-if="!groupStore.loading && tableData.length === 0"
        description="还没有分组"
        class="table-empty"
      />
    </div>
  </div>
</template>

<style scoped>
.group-manage-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.page-head {
  flex-shrink: 0;
}

.create-card {
  padding: 14px 16px;
  flex-shrink: 0;
}

.table-card {
  flex: 1;
  min-height: 0;
  padding: 0 12px 12px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.card-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  padding: 12px 0;
  flex-shrink: 0;
}
.table-empty {
  position: absolute;
}

/* 行内重命名触发区 */
:deep(.group-name) {
  cursor: text;
  padding: 2px 6px;
  border-radius: 3px;
  transition: background 0.12s;
}
:deep(.group-name:hover) {
  background: var(--sidebar-hover-bg);
}
</style>
