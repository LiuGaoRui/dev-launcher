<script setup lang="ts">
// 项目扫描向导 Dialog。
//
// 设计：
// - 两段式：① 选根目录 → 扫描；② 勾选检测到的项目 + 选启动方案 → 批量添加
// - 扫描结果用 NDataTable 展示，每行可勾选、可切换启动方案、可改端口
// - 添加时把勾选项的 (detected + 选中方案) 转 ProjectInput，逐个调 store.add()
// - 「手动添加」按钮打开原 ProjectFormDialog（由父组件监听 manual 事件处理）
//
// 与 ProjectFormDialog 的关系：
//   本组件只负责「从根目录扫描批量添加」；单项目精修仍用 ProjectFormDialog（edit 模式）。

import { computed, h, reactive, ref, watch } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import {
  NModal,
  NButton,
  NInput,
  NInputGroup,
  NDataTable,
  NSelect,
  NTag,
  NSpin,
  NEmpty,
  NCheckbox,
  useMessage,
  type DataTableColumns,
} from 'naive-ui'
import { scanProjects } from '@/api/detect'
import { PROJECT_TYPE_LABELS, type ProjectInput, type ProjectType } from '@/types/project'
import { parsePorts } from '@/utils/ports'
import type { DetectedProject, LaunchScheme } from '@/types/detect'
import { useProjectStore } from '@/stores/project'

const props = defineProps<{
  modelValue: boolean
  /** 当前分组 tab（添加时作为新项目的 group_id） */
  defaultGroupId?: number | null
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  /** 用户点「手动添加」——父组件打开原 ProjectFormDialog */
  manual: []
  /** 批量添加完成 */
  done: [addedCount: number]
}>()

const message = useMessage()
const projectStore = useProjectStore()

const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v),
})

// ===== 扫描状态 =====

/** 选中的根目录 */
const rootPath = ref('')
/** 扫描中 */
const scanning = ref(false)
/** 扫描结果 */
const detected = ref<DetectedProject[]>([])

/** 每行的可编辑状态：选中的方案 start_cmd（key 用 path 去重）、端口输入 */
interface RowState {
  /** 选中的方案在 schemes 中的下标 */
  schemeIndex: number
  /** 端口输入（逗号分隔字符串，便于编辑） */
  ports: string
}
const rowStates = reactive<Record<string, RowState>>({})

/** 勾选的行 path 集合 */
const checkedPaths = ref<Set<string>>(new Set())

// dialog 打开时重置状态
watch(
  () => props.modelValue,
  (open) => {
    if (!open) return
    rootPath.value = ''
    detected.value = []
    Object.keys(rowStates).forEach((k) => delete rowStates[k])
    checkedPaths.value = new Set()
  },
)

// ===== 目录选择 =====

async function pickDirectory() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: '选择要扫描的根目录',
      defaultPath: rootPath.value || undefined,
    })
    if (typeof selected === 'string' && selected.length > 0) {
      rootPath.value = selected
    }
  } catch (e) {
    console.debug('pick directory canceled or failed:', e)
  }
}

// ===== 扫描 =====

async function handleScan() {
  if (!rootPath.value.trim()) {
    message.warning('请先选择根目录')
    return
  }
  scanning.value = true
  detected.value = []
  Object.keys(rowStates).forEach((k) => delete rowStates[k])
  checkedPaths.value = new Set()
  try {
    const list = await scanProjects(rootPath.value.trim())
    detected.value = list
    // 初始化每行状态：默认选 recommended 方案（否则第一个），端口预填
    for (const d of list) {
      const recIdx = d.schemes.findIndex((s) => s.recommended)
      rowStates[d.path] = {
        schemeIndex: recIdx >= 0 ? recIdx : 0,
        ports: d.expected_ports.join(', '),
      }
      // 默认全选
      checkedPaths.value.add(d.path)
    }
  } catch (e) {
    message.error(`扫描失败：${e instanceof Error ? e.message : String(e)}`)
  } finally {
    scanning.value = false
  }
}

// ===== 全选 / 反选 =====

const allChecked = computed(
  () => detected.value.length > 0 && detected.value.every((d) => checkedPaths.value.has(d.path)),
)
const someChecked = computed(
  () =>
    detected.value.some((d) => checkedPaths.value.has(d.path)) && !allChecked.value,
)

function toggleAll(on: boolean) {
  if (on) {
    checkedPaths.value = new Set(detected.value.map((d) => d.path))
  } else {
    checkedPaths.value = new Set()
  }
}

function toggleRow(path: string, on: boolean) {
  const next = new Set(checkedPaths.value)
  if (on) next.add(path)
  else next.delete(path)
  checkedPaths.value = next
}

const checkedCount = computed(() => checkedPaths.value.size)

// ===== 方案下拉 =====

function schemeOptions(row: DetectedProject) {
  return row.schemes.map((s, i) => ({
    label: `[${s.label}] ${s.start_cmd}`,
    value: i,
  }))
}

// ===== 添加 =====

const adding = ref(false)

/** 把一行 (detected + 选中方案) 转 ProjectInput */
function buildInput(d: DetectedProject): ProjectInput | null {
  const st = rowStates[d.path]
  if (!st) return null
  const scheme: LaunchScheme = d.schemes[st.schemeIndex]
  const ports = parsePorts(st.ports)
  return {
    name: d.name,
    group_id: props.defaultGroupId ?? null,
    type: d.type,
    path: d.path,
    // workdir 默认用扫描根目录（license 等运行时资源在此）
    workdir: d.workdir || null,
    start_cmd: scheme.start_cmd,
    build_cmd: scheme.build_cmd,
    expected_ports: ports,
    enabled: true,
  }
}

async function handleAdd() {
  if (checkedCount.value === 0) {
    message.warning('请至少勾选一个项目')
    return
  }
  adding.value = true
  let ok = 0
  let fail = 0
  const targets = detected.value.filter((d) => checkedPaths.value.has(d.path))
  for (const d of targets) {
    const input = buildInput(d)
    if (!input) continue
    const [, err] = await projectStore.safe(() => projectStore.add(input))
    if (err) {
      fail++
      message.error(`「${input.name}」添加失败：${err}`)
    } else {
      ok++
    }
  }
  adding.value = false
  if (ok > 0) {
    emit('done', ok)
    visible.value = false
  } else if (fail > 0) {
    message.error(`全部 ${fail} 个项目添加失败`)
  }
}

// ===== 表格列定义 =====

const columns = computed<DataTableColumns<DetectedProject>>(() => [
  {
    title: () =>
      hCheckbox({
        checked: allChecked.value,
        indeterminate: someChecked.value,
        onUpdate: (v: boolean) => toggleAll(v),
      }),
    key: 'check',
    width: 44,
    render: (row) =>
      hCheckbox({
        checked: checkedPaths.value.has(row.path),
        onUpdate: (v: boolean) => toggleRow(row.path, v),
      }),
  },
  { title: '项目名', key: 'name', width: 130, ellipsis: { tooltip: true } },
  {
    title: '类型',
    key: 'type',
    width: 90,
    render: (row) =>
      hTag(row.type),
  },
  {
    title: '路径',
    key: 'rel_path',
    ellipsis: { tooltip: true },
    render: (row) => hPath(row.rel_path),
  },
  {
    title: '启动方案',
    key: 'scheme',
    width: 240,
    render: (row) =>
      hScheme(row),
  },
  {
    title: '预期端口',
    key: 'ports',
    width: 130,
    render: (row) => hPorts(row),
  },
])

// ===== render helpers（用 h() 避免模板复杂度） =====

function hCheckbox(p: { checked: boolean; indeterminate?: boolean; onUpdate: (v: boolean) => void }) {
  return h(NCheckbox, {
    checked: p.checked,
    indeterminate: p.indeterminate ?? false,
    'onUpdate:checked': p.onUpdate,
  })
}

function hTag(type: ProjectType) {
  return h(
    NTag,
    { size: 'small', type: type === 'node' ? 'success' : type === 'springboot' ? 'info' : 'warning', bordered: false },
    { default: () => PROJECT_TYPE_LABELS[type] },
  )
}

function hPath(rel: string) {
  return h('span', { class: 'cell-path' }, rel)
}

function hScheme(row: DetectedProject) {
  return h(NSelect, {
    size: 'small',
    value: rowStates[row.path]?.schemeIndex ?? 0,
    options: schemeOptions(row),
    'onUpdate:value': (v: number) => {
      if (rowStates[row.path]) rowStates[row.path].schemeIndex = v
    },
  })
}

function hPorts(row: DetectedProject) {
  return h(NInput, {
    size: 'small',
    value: rowStates[row.path]?.ports ?? '',
    placeholder: '端口',
    'onUpdate:value': (v: string) => {
      if (rowStates[row.path]) rowStates[row.path].ports = v
    },
  })
}

function handleManual() {
  emit('manual')
}

function handleClose() {
  visible.value = false
}
</script>

<template>
  <NModal
    v-model:show="visible"
    preset="card"
    title="扫描添加项目"
    style="width: 860px"
    :mask-closable="false"
  >
    <!-- 目录选择区 -->
    <div class="scan-bar">
      <NInputGroup>
        <NInput
          v-model:value="rootPath"
          placeholder="选择要扫描的根目录（如代码仓库根）"
          readonly
          style="flex: 1"
        />
        <NButton @click="pickDirectory">选择...</NButton>
      </NInputGroup>
      <NButton type="primary" :loading="scanning" :disabled="!rootPath" @click="handleScan">
        扫描
      </NButton>
    </div>

    <!-- 结果区 -->
    <div class="result-area">
      <div v-if="scanning" class="result-loading">
        <NSpin size="small" />
        <span class="loading-text">正在扫描...</span>
      </div>

      <NDataTable
        v-else-if="detected.length"
        :columns="columns"
        :data="detected"
        :bordered="false"
        :single-line="false"
        size="small"
        :max-height="360"
      />

      <NEmpty
        v-else-if="rootPath && !scanning"
        description="选择根目录并点击「扫描」，将自动检测其中的 Java / Node 项目"
        class="result-empty"
      />
    </div>

    <template #footer>
      <div class="footer">
        <NButton @click="handleManual">手动添加</NButton>
        <div class="footer-right">
          <span v-if="checkedCount" class="checked-count">已选 {{ checkedCount }} 项</span>
          <NButton @click="handleClose">取消</NButton>
          <NButton
            type="primary"
            :loading="adding"
            :disabled="!checkedCount"
            @click="handleAdd"
          >
            添加{{ checkedCount ? ` ${checkedCount} 个` : '' }}
          </NButton>
        </div>
      </div>
    </template>
  </NModal>
</template>

<style scoped>
.scan-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.result-area {
  min-height: 120px;
}
.result-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 120px;
}
.loading-text {
  font-size: 13px;
  color: var(--text-tertiary);
}
.result-empty {
  height: 120px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.cell-path {
  font-size: 12px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
}

.footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}
.footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.checked-count {
  font-size: 13px;
  color: var(--text-tertiary);
  margin-right: 4px;
}
</style>
