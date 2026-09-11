<script setup lang="ts">
// 项目列表页 —— 阶段 3 核心 UI。
//
// 布局：顶部工具栏 + 扁平卡片网格（所有目录的项目按顺序排列在同一网格）。
//   同一 scan_root 目录的卡片标题栏使用相同背景色，不同目录不同色，以视觉区分。
//   目录→色相 用 FNV-1a 哈希稳定绑定，拖拽改变顺序或新增目录都不影响已有目录颜色。
// 能力：新建/编辑/删除项目、启动/停止、构建（后台）、日志、点击访问链接用默认浏览器打开。
// 排序：卡片可跨目录任意拖拽排序（持久化全局 sort_order）。
// 编排：所有 store 操作在本页面集中进行，卡片组件无状态。

import { onActivated, onDeactivated, onMounted, reactive, ref } from 'vue'
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

// ===== 目录色（同 scan_root 同色） =====
//
// 剔除 0(红)/340(粉) 等与运行状态色（绿/橙/灰）、错误色语义冲突的色相。
// 用 FNV-1a 对 scan_root 字符串哈希后取下标，保证「目录→色相」稳定绑定：
// 拖拽改变顺序、新增/删除其他目录都不影响本目录的颜色。
const HUES = [210, 28, 145, 270, 190, 50, 310, 95, 230, 125]
function hueOf(root: string | null): number {
  const s = root ?? '__other__'
  let h = 2166136261
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return HUES[Math.abs(h) % HUES.length]
}

// ===== 卡片拖拽排序（跨目录任意排序） =====
//
// 卡片根元素自带 draggable + data-project-id（见 ProjectCard.vue），
// 这里用事件委托在 grid 容器上监听 drag 事件，解析 data-project-id。

/** 拖动源：项目 id */
const dragSource = ref<number | null>(null)
/** 当前被高亮的放置目标 id */
const dragOverTarget = ref<number | null>(null)

/** 判断某卡片是否处于拖动中（半透明视觉态） */
function isCardDragging(id: number): boolean {
  return dragSource.value === id
}
/** 判断某卡片是否为放置目标（高亮边框） */
function isCardDragOver(id: number): boolean {
  return dragOverTarget.value === id
}

/** 从拖拽事件目标向上找最近的卡片，取其 data-project-id */
function cardIdFromEvent(e: DragEvent): number | null {
  const el = (e.target as HTMLElement)?.closest('[data-project-id]')
  if (!el) return null
  const id = Number(el.getAttribute('data-project-id'))
  return Number.isFinite(id) ? id : null
}

function onCardDragStart(e: DragEvent, projectId: number) {
  dragSource.value = projectId
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    e.dataTransfer.setData('text/plain', `card:${projectId}`)
  }
}

function onCardDragOver(e: DragEvent, projectId: number) {
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  if (dragSource.value !== projectId) dragOverTarget.value = projectId
}

async function onCardDrop(e: DragEvent, targetId: number) {
  e.preventDefault()
  e.stopPropagation()
  if (dragSource.value === null) {
    resetDrag()
    return
  }
  const sourceId = dragSource.value
  if (sourceId !== targetId) {
    await reorderCards(sourceId, targetId)
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

function onGridDrop(e: DragEvent) {
  const id = cardIdFromEvent(e)
  if (id === null) {
    resetDrag()
    return
  }
  onCardDrop(e, id)
}

/** 把源卡片移到目标卡片位置（全局重排），其余顺延，并持久化新顺序 */
async function reorderCards(sourceId: number, targetId: number) {
  const ids = projectStore.projects.map((p) => p.id)
  if (!moveInArray(ids, ids.indexOf(sourceId), ids.indexOf(targetId))) return
  const [, err] = await projectStore.safe(() => projectStore.reorderProjectsOrder(ids))
  if (err) message.error(`调整顺序失败：${err}`)
}

/** 拖拽结束 / 离开：清空状态 */
function resetDrag() {
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

/** 停止构建：后端 taskkill 杀构建进程树（构建中卡片按钮自动切换为「停止构建」） */
function handleStopBuild(p: Project) {
  void buildStore.stopBuild(p.id)
}

/**
 * 点击「重启」：停止 → 构建 → 启动 三步串行。
 * - 先停再构建：旧进程会占着 build 产物文件（如 dist/、jar，Windows 文件锁）
 *   和端口；先停掉才能让 build 干净覆盖产物，start 才不会起旧/半成品产物。
 * - 停止仅在项目运行时执行（已停止则跳过）。后端 stop_project 已轮询确认端口/PID 释放。
 * - 构建仅在配置了 build_cmd 时执行（等待完成，成功才继续），失败则中止（buildStore 已提示，不重复报错）。
 * - 全程用 withBusy 锁定卡片按钮；构建等待带超时兜底，路由切换触发 clearAllTimers 时也不会永久挂起。
 */
async function handleRestart(p: Project) {
  /** 等待构建完成；带超时兜底，避免 buildStore 在路由切换 clearAllTimers 时删除 entry 导致 Promise 永久挂起。 */
  function waitBuildDone(id: number, timeoutMs = 10 * 60 * 1000): Promise<boolean> {
    return new Promise<boolean>((resolve) => {
      let settled = false
      const done = (exitCode: number) => {
        if (settled) return
        settled = true
        resolve(exitCode === 0)
      }
      const timer = setTimeout(() => done(-1), timeoutMs)
      buildStore.startBuild(id, (exitCode) => {
        clearTimeout(timer)
        done(exitCode)
      })
    })
  }

  const [, err] = await withBusy(p.id, async () => {
    // 1. 停止（仅当运行中）—— 必须先停，释放产物文件锁 + 端口，否则 build 覆盖失败
    if (projectStore.isRunning(p.id)) {
      const [, stopErr] = await projectStore.safe(() => projectStore.stop(p.id))
      if (stopErr) {
        message.error(`「${p.name}」重启失败：停止出错 ${stopErr}`)
        return false
      }
    }

    // 2. 构建（仅当配置了 build_cmd；等待完成，成功才继续）
    //    无 build_cmd 时静默跳过构建，直接进入启动步骤
    if (p.build_cmd?.trim()) {
      const buildOk = await waitBuildDone(p.id)
      if (!buildOk) {
        // 先停后建：构建失败时项目已被停止，需明确告知（buildStore 已提示构建失败，此处补足停机信息）
        message.error(`「${p.name}」构建失败，项目已停止，请处理后重试`)
        return false
      }
    }

    // 3. 启动（后端 stop 已确认 PID/端口释放，无需前端重试兜底）
    const [, startErr] = await projectStore.safe(() => projectStore.start(p.id))
    if (startErr) {
      message.error(`「${p.name}」重启失败：启动出错 ${startErr}`)
      return false
    }
    // start 返回 Ok 即成功（后端 start_project 已做端口预检 + spawn）；
    // 不再依赖即时 probe 判断"状态未就绪"——受 probing 守卫影响可能读到过期状态，造成误报
    return true
  })
  if (err) message.error(`「${p.name}」重启异常：${err}`)
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

    <!-- 扁平卡片网格（所有目录的项目按顺序排列，同目录同色） -->
    <div v-if="projectStore.loading" class="grid-loading">
      <NSpin size="small" />
    </div>

    <div
      v-else-if="projectStore.projects.length"
      class="grid"
      @dragstart="onGridDragStart($event)"
      @dragover="onGridDragOver($event)"
      @drop="onGridDrop($event)"
      @dragend="onDragEnd"
    >
      <ProjectCard
        v-for="p in projectStore.projects"
        :key="p.id"
        :project="p"
        :status="projectStore.statuses[p.id] ?? null"
        :busy="busyIds.has(p.id)"
        :build-state="buildStore.getState(p.id)"
        :dragging="isCardDragging(p.id)"
        :drag-over="isCardDragOver(p.id)"
        :group-hue="hueOf(p.scan_root)"
        @start="handleStart(p)"
        @stop="handleStop(p)"
        @restart="handleRestart(p)"
        @build="handleBuild(p)"
        @stop-build="handleStopBuild(p)"
        @log="handleLog(p)"
        @edit="openEdit(p)"
        @delete="handleDelete(p)"
        @open-url="handleOpenUrl"
      />
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

/* 卡片网格（1280px 窗口宽度下一行 4 列，窗口缩到 minWidth 960 时降为 3 列） */
.grid {
  flex: 1;
  min-height: 0;
  overflow: auto;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(290px, 1fr));
  gap: 12px;
  align-content: start;
  padding-right: 4px;
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
