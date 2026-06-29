<script setup lang="ts">
// 项目列表页 —— 阶段 3 核心 UI。
//
// 布局：顶部工具栏 + 按「扫描目录」分组的程序面板。
//   每个扫描目录一个面板标题头（显示扫描目录路径 + 项目数），下方放该目录扫描出的程序卡片。
//   手动添加、无 scan_root 的项目归入「其他」面板。
// 能力：新建/编辑/删除项目、启动/停止、构建（后台）、日志、点击访问链接用默认浏览器打开。
// 排序：面板标题头可拖拽调整扫描目录顺序（持久化）；卡片可在同面板内拖拽排序（持久化）。
// 编排：所有 store 操作在本页面集中进行，卡片组件无状态。

import { computed, onActivated, onDeactivated, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { NButton, NEmpty, NSpin, useMessage, useDialog } from 'naive-ui'
import { useProjectStore } from '@/stores/project'
import { useBuildStore } from '@/stores/build'
import { openUrl } from '@/api/system'
import { moveInArray } from '@/utils/array'
import type { Project, ProjectInput } from '@/types/project'
import ProjectCard from '@/components/project/ProjectCard.vue'
import ProjectFormDialog from '@/components/project/ProjectFormDialog.vue'
import ProjectScanDialog from '@/components/project/ProjectScanDialog.vue'

defineOptions({ name: 'ProjectList' })

const projectStore = useProjectStore()
const buildStore = useBuildStore()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()

// ===== 按扫描目录分组 =====

/** 单个面板的数据：一个扫描目录下的全部项目 */
interface Panel {
  /** 分组键（scan_root，或 '__other__' 表示无扫描目录的项目） */
  key: string
  /** 面板标题展示文本（扫描目录路径，或「其他」） */
  title: string
  /** 是否「其他」分组（无 scan_root） */
  isOther: boolean
  /** 该面板下的项目 */
  projects: Project[]
}

/**
 * 按 scan_root 把项目聚合为面板列表。
 *
 * - 有 scan_root 的项目按目录聚合
 * - 面板排序：优先按 store.scanRootOrder（用户拖拽过的顺序），
 *   未记录的目录按字典序兜底排到已记录目录之后
 * - 无 scan_root 的项目统一归入「其他」面板，恒在末尾
 * - 面板内项目按 sort_order ASC 排序（后端已排好，此处保持稳定）
 */
const panels = computed<Panel[]>(() => {
  const map = new Map<string, Project[]>()
  const others: Project[] = []
  for (const p of projectStore.projects) {
    const root = p.scan_root
    if (root) {
      const arr = map.get(root)
      if (arr) arr.push(p)
      else map.set(root, [p])
    } else {
      others.push(p)
    }
  }

  // 有 scan_root 的面板：先按 scanRootOrder 排序，未记录的按字典序兜底
  const order = projectStore.scanRootOrder
  const sortedRoots = Array.from(map.keys()).sort((a, b) => {
    const oa = order[a]
    const ob = order[b]
    // 两者都有记录 → 按 sort_order
    if (oa !== undefined && ob !== undefined) return oa - ob
    // 仅 a 有记录 → a 在前
    if (oa !== undefined) return -1
    // 仅 b 有记录 → b 在前
    if (ob !== undefined) return 1
    // 都无记录 → 按字典序
    return a.localeCompare(b, 'zh')
  })

  const panels: Panel[] = sortedRoots.map((root) => ({
    key: root,
    title: root,
    isOther: false,
    projects: map.get(root)!,
  }))
  // 「其他」面板放最后
  if (others.length) {
    panels.push({
      key: '__other__',
      title: '其他',
      isOther: true,
      projects: others,
    })
  }
  return panels
})

// ===== 拖拽排序状态 =====
//
// 两类拖拽共享一套状态：拖动源 + 放置高亮目标。
// dragKind 区分当前是「面板」还是「卡片」拖拽，避免误判。

type DragKind = 'panel' | 'card'
const dragKind = ref<DragKind | null>(null)
/** 拖动源：面板 key（面板拖拽）或项目 id（卡片拖拽） */
const dragSource = ref<string | number | null>(null)
/** 当前被高亮的放置目标 key（面板拖拽）或项目 id（卡片拖拽） */
const dragOverTarget = ref<string | number | null>(null)

/** 判断某卡片是否处于拖动中（半透明视觉态） */
function isCardDragging(id: number): boolean {
  return dragKind.value === 'card' && dragSource.value === id
}
/** 判断某卡片是否为放置目标（高亮边框） */
function isCardDragOver(id: number): boolean {
  return dragKind.value === 'card' && dragOverTarget.value === id
}
/** 判断某面板标题头是否处于拖动中 */
function isPanelDragging(key: string): boolean {
  return dragKind.value === 'panel' && dragSource.value === key
}
/** 判断某面板是否为放置目标 */
function isPanelDragOver(key: string): boolean {
  return dragKind.value === 'panel' && dragOverTarget.value === key
}

// ===== 面板拖拽（调整扫描目录顺序） =====

function onPanelDragStart(e: DragEvent, key: string) {
  dragKind.value = 'panel'
  dragSource.value = key
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    // 必须 setData 否部分浏览器不触发 dragover
    e.dataTransfer.setData('text/plain', `panel:${key}`)
  }
}

function onPanelDragOver(e: DragEvent, key: string) {
  if (dragKind.value !== 'panel') return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  if (dragSource.value !== key) dragOverTarget.value = key
}

async function onPanelDrop(e: DragEvent, targetKey: string) {
  e.preventDefault()
  e.stopPropagation()
  if (dragKind.value !== 'panel' || dragSource.value === null) {
    resetDrag()
    return
  }
  const sourceKey = dragSource.value as string
  if (sourceKey !== targetKey) {
    await reorderPanels(sourceKey, targetKey)
  }
  resetDrag()
}

/** 把源面板移到目标面板的位置（其余顺延） */
async function reorderPanels(sourceKey: string, targetKey: string) {
  // 「其他」面板不可拖拽也不作为持久化目标（它无 scan_root）
  if (sourceKey === '__other__' || targetKey === '__other__') return
  const keys = panels.value.filter((p) => !p.isOther).map((p) => p.key)
  if (!moveInArray(keys, keys.indexOf(sourceKey), keys.indexOf(targetKey))) return
  const [, err] = await projectStore.safe(() =>
    projectStore.reorderScanRootsOrder(keys),
  )
  if (err) message.error(`调整顺序失败：${err}`)
}

// ===== 卡片拖拽（同面板内排序） =====
//
// 卡片根元素自带 draggable + data-project-id（见 ProjectCard.vue），
// 这里用事件委托在 grid 容器上监听 drag 事件，解析 data-project-id。

/** 从拖拽事件目标向上找最近的卡片，取其 data-project-id */
function cardIdFromEvent(e: DragEvent): number | null {
  const el = (e.target as HTMLElement)?.closest('[data-project-id]')
  if (!el) return null
  const id = Number(el.getAttribute('data-project-id'))
  return Number.isFinite(id) ? id : null
}

function onCardDragStart(e: DragEvent, projectId: number) {
  dragKind.value = 'card'
  dragSource.value = projectId
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', `card:${projectId}`)
  }
}

function onCardDragOver(e: DragEvent, projectId: number) {
  if (dragKind.value !== 'card') return
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  if (dragSource.value !== projectId) dragOverTarget.value = projectId
}

async function onCardDrop(e: DragEvent, panel: Panel, targetId: number) {
  e.preventDefault()
  e.stopPropagation()
  if (dragKind.value !== 'card' || dragSource.value === null) {
    resetDrag()
    return
  }
  const sourceId = dragSource.value as number
  if (sourceId !== targetId) {
    await reorderCardsInPanel(panel, sourceId, targetId)
  }
  resetDrag()
}

// ---- grid 容器事件委托入口（从冒泡事件解析目标卡片 id） ----

function onGridDragStart(e: DragEvent) {
  const id = cardIdFromEvent(e)
  if (id === null) return
  onCardDragStart(e, id)
}

function onGridDragOver(e: DragEvent) {
  const id = cardIdFromEvent(e)
  if (id === null) return
  onCardDragOver(e, id)
}

function onGridDrop(e: DragEvent, panel: Panel) {
  const id = cardIdFromEvent(e)
  if (id === null) {
    resetDrag()
    return
  }
  onCardDrop(e, panel, id)
}

/** 把源卡片移到目标卡片位置（同面板内），其余顺延，并持久化新顺序 */
async function reorderCardsInPanel(
  panel: Panel,
  sourceId: number,
  targetId: number,
) {
  const ids = panel.projects.map((p) => p.id)
  if (!moveInArray(ids, ids.indexOf(sourceId), ids.indexOf(targetId))) return
  const [, err] = await projectStore.safe(() =>
    projectStore.reorderProjectsOrder(ids),
  )
  if (err) message.error(`调整顺序失败：${err}`)
}

/** 拖拽结束 / 离开：清空状态 */
function resetDrag() {
  dragKind.value = null
  dragSource.value = null
  dragOverTarget.value = null
}
function onDragEnd() {
  resetDrag()
}

// ===== 启停操作的 busy 跟踪（按项目 id） =====

const busyIds = reactive<Set<number>>(new Set())
function setBusy(id: number, on: boolean) {
  if (on) busyIds.add(id)
  else busyIds.delete(id)
}

async function withBusy<T>(id: number, fn: () => Promise<T>): Promise<[T | null, string | null]> {
  setBusy(id, true)
  const result = await projectStore.safe(fn)
  setBusy(id, false)
  return result
}

// ===== 数据加载 =====

async function loadAll() {
  await projectStore.fetchAll()
}

// 加载数据并启动监控轮询（KeepAlive 感知：激活时开始，暂停时停止）
onMounted(async () => {
  await loadAll()
})
onActivated(() => {
  projectStore.startPolling()
})
onDeactivated(() => {
  projectStore.stopPolling()
  buildStore.clearAllTimers()
})

// ===== 启停 / 日志 / 构建 =====

/** 点击「日志」按钮：进入日志详情页 */
function handleLog(p: Project) {
  router.push({ name: 'ProjectDetail', params: { id: p.id } })
}

async function handleStart(p: Project) {
  const [, err] = await withBusy(p.id, () => projectStore.start(p.id))
  if (err) message.error(`启动失败：${err}`)
}

async function handleStop(p: Project) {
  const [, err] = await withBusy(p.id, () => projectStore.stop(p.id))
  if (err) message.error(`停止失败：${err}`)
}

/** 点击访问链接：用系统默认浏览器打开 */
async function handleOpenUrl(url: string) {
  const [, err] = await projectStore.safe(() => openUrl(url))
  if (err) message.error(`打开链接失败：${err}`)
}

/** 后台构建：不弹框、不设 busy，构建按钮由 buildStore 状态驱动图标。
 *  支持多项目同时构建。 */
function handleBuild(p: Project) {
  if (!p.build_cmd?.trim()) {
    message.warning('该项目未配置构建命令')
    return
  }
  void buildStore.startBuild(p.id)
}

// ===== 删除 =====

async function handleDelete(p: Project) {
  dialog.warning({
    title: '删除确认',
    content: `确定删除项目「${p.name}」吗？此操作不可恢复。`,
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      const [, err] = await projectStore.safe(() => projectStore.remove(p.id))
      if (err) message.error(`删除失败：${err}`)
    },
  })
}

// ===== 新建/编辑 Dialog =====

// 扫描向导弹窗
const scanVisible = ref(false)
// 单项目表单弹窗（手动添加 / 编辑）
const dialogVisible = ref(false)
const editingProject = ref<Project | null>(null)
const submitting = ref(false)

function openCreate() {
  // 「+ 新建项目」打开扫描向导
  scanVisible.value = true
}

/** 扫描弹窗内点「手动添加」→ 打开原表单（create 模式空表单） */
function handleManualCreate() {
  editingProject.value = null
  dialogVisible.value = true
}

function openEdit(p: Project) {
  editingProject.value = p
  dialogVisible.value = true
}

async function handleSubmitForm(input: ProjectInput, origin: Project | null) {
  submitting.value = true
  if (origin) {
    const [, err] = await projectStore.safe(() => projectStore.patch(origin.id, input))
    submitting.value = false
    if (err) {
      message.error(`保存失败：${err}`)
      return
    }
    dialogVisible.value = false
  } else {
    const [created, err] = await projectStore.safe(() => projectStore.add(input))
    submitting.value = false
    if (err || !created) {
      message.error(`创建失败：${err}`)
      return
    }
    dialogVisible.value = false
  }
}

// ===== 批量操作（全部启动/停止） =====

async function startAll() {
  const targets = projectStore.projects.filter((p) => !projectStore.isRunning(p.id))
  if (!targets.length) return
  const results = await Promise.allSettled(
    targets.map((p) => projectStore.safe(() => projectStore.start(p.id))),
  )
  results.forEach((r, i) => {
    if (r.status === 'fulfilled' && r.value[1]) message.error(`「${targets[i].name}」启动失败：${r.value[1]}`)
  })
  await projectStore.probeNow()
}

async function stopAll() {
  const targets = projectStore.projects.filter((p) => projectStore.isRunning(p.id))
  if (!targets.length) return
  const results = await Promise.allSettled(
    targets.map((p) => projectStore.safe(() => projectStore.stop(p.id))),
  )
  results.forEach((r, i) => {
    if (r.status === 'fulfilled' && r.value[1]) message.error(`「${targets[i].name}」停止失败：${r.value[1]}`)
  })
  await projectStore.probeNow()
}
</script>

<template>
  <div class="project-list-page">
    <!-- 顶部工具栏 -->
    <div class="toolbar">
      <div class="toolbar-title">
        <span class="page-name">项目</span>
        <span class="page-count">{{ projectStore.projects.length }}</span>
      </div>
      <div class="toolbar-actions">
        <NButton size="small" secondary @click="startAll">全部启动</NButton>
        <NButton size="small" secondary @click="stopAll">全部停止</NButton>
        <NButton size="small" secondary @click="projectStore.probeNow()">刷新</NButton>
        <NButton size="small" type="primary" @click="openCreate">+ 新建项目</NButton>
      </div>
    </div>

    <!-- 按扫描目录分组的面板 -->
    <div v-if="projectStore.loading" class="grid-loading">
      <NSpin size="small" />
    </div>

    <div v-else-if="panels.length" class="panels">
      <section
        v-for="panel in panels"
        :key="panel.key"
        class="panel"
      >
        <!-- 面板标题头（可拖拽调整扫描目录顺序；「其他」面板不可拖） -->
        <header
          class="panel-head"
          :class="{
            'panel-head--other': panel.isOther,
            'panel-dragging': isPanelDragging(panel.key),
            'panel-drag-over': isPanelDragOver(panel.key),
          }"
          :draggable="!panel.isOther"
          @dragstart="onPanelDragStart($event, panel.key)"
          @dragover="onPanelDragOver($event, panel.key)"
          @drop="onPanelDrop($event, panel.key)"
          @dragend="onDragEnd"
        >
          <span class="drag-handle" :title="panel.isOther ? '' : '拖拽调整顺序'">⠿</span>
          <span class="panel-title" :title="panel.title">{{ panel.title }}</span>
          <span class="panel-count">{{ panel.projects.length }}</span>
        </header>

        <!-- 该目录下的程序卡片（拖拽事件委托到 grid 容器） -->
        <div
          class="grid"
          @dragstart="onGridDragStart($event)"
          @dragover="onGridDragOver($event)"
          @drop="onGridDrop($event, panel)"
          @dragend="onDragEnd"
        >
          <ProjectCard
            v-for="p in panel.projects"
            :key="p.id"
            :project="p"
            :status="projectStore.statuses[p.id] ?? null"
            :busy="busyIds.has(p.id)"
            :build-state="buildStore.getState(p.id)"
            :dragging="isCardDragging(p.id)"
            :drag-over="isCardDragOver(p.id)"
            @start="handleStart(p)"
            @stop="handleStop(p)"
            @build="handleBuild(p)"
            @log="handleLog(p)"
            @edit="openEdit(p)"
            @delete="handleDelete(p)"
            @open-url="handleOpenUrl"
          />
        </div>
      </section>
    </div>

    <NEmpty
      v-else
      description="还没有项目，点击右上角「新建项目」开始"
      class="empty-state"
    />

    <!-- 扫描向导（点「+ 新建项目」打开） -->
    <ProjectScanDialog v-model="scanVisible" @manual="handleManualCreate" />

    <!-- 新建/编辑对话框（手动添加 / 编辑已有项目） -->
    <ProjectFormDialog
      v-model="dialogVisible"
      :project="editingProject"
      :submitting="submitting"
      @submit="handleSubmitForm"
    />
  </div>
</template>

<style scoped>
.project-list-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

/* 顶部工具栏 */
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  flex-shrink: 0;
}
.toolbar-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.page-count {
  font-size: 13px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
}
.toolbar-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

/* 面板滚动容器 */
.panels {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-right: 4px;
}

/* 单个面板 */
.panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 面板标题头（可拖拽） */
.panel-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 4px;
  border-bottom: 1px solid var(--divider);
  flex-shrink: 0;
  user-select: none;
}
.panel-head:not(.panel-head--other) {
  cursor: grab;
}
.panel-head:not(.panel-head--other):active {
  cursor: grabbing;
}
.panel-dragging {
  opacity: 0.45;
}
.panel-drag-over {
  border-bottom-color: var(--accent);
  background: var(--card-hover-bg);
}
.drag-handle {
  color: var(--text-tertiary);
  font-size: 14px;
  line-height: 1;
  cursor: inherit;
  opacity: 0.7;
}
.panel-head--other .drag-handle {
  visibility: hidden;
}
.panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  font-family: var(--code-font);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.panel-head--other .panel-title {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
}
.panel-count {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
  background: var(--card-bg);
  border: 1px solid var(--divider);
  border-radius: 9px;
  padding: 0 7px;
  line-height: 16px;
  flex-shrink: 0;
}

/* 卡片网格 */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(330px, 1fr));
  gap: 12px;
  align-content: start;
}
.grid-loading {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
