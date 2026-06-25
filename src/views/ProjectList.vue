<script setup lang="ts">
// 项目列表页 —— 阶段 3 核心 UI。
//
// 布局：左侧分组筛选 tab（全部 / 未分组 / 各分组） + 右侧项目卡片网格。
// 能力：新建/编辑/删除项目、启动/停止/重启、按分组筛选。
// 编排：所有 store 操作在本页面集中进行，卡片组件无状态。

import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useGroupStore } from '@/stores/group'
import { useProjectStore } from '@/stores/project'
import { useBuildStore } from '@/stores/build'
import type { Project, ProjectInput } from '@/types/project'
import ProjectCard from '@/components/project/ProjectCard.vue'
import ProjectFormDialog from '@/components/project/ProjectFormDialog.vue'
import BuildDialog from '@/components/project/BuildDialog.vue'

const groupStore = useGroupStore()
const projectStore = useProjectStore()
const buildStore = useBuildStore()
const router = useRouter()

// ===== 筛选 =====

/** 当前选中的分组 id：null=全部，-1=未分组，其它=分组 id */
const activeGroup = ref<number | null>(null)

/** 筛选 tab 选项（全部 / 未分组 / 各分组） */
const groupTabs = computed(() => {
  const base = [
    { id: null as number | null, name: '全部' },
    { id: -1, name: '未分组' },
  ]
  return base.concat(
    groupStore.groups.map((g) => ({ id: g.id, name: g.name })),
  )
})

/** 按分组 id 过滤项目 */
function projectsByGroup(id: number | null): Project[] {
  if (id === null) return projectStore.projects
  if (id === -1) return projectStore.projects.filter((p) => p.group_id === null)
  return projectStore.projects.filter((p) => p.group_id === id)
}

/** 当前 tab 下显示的项目（前端过滤，数据量不大无需每次查 DB） */
const filteredProjects = computed(() => projectsByGroup(activeGroup.value))

function tabCount(id: number | null): number {
  if (id === null) return projectStore.projects.length
  return projectsByGroup(id).length
}

// ===== 启停操作的 busy 跟踪（按项目 id） =====

const busyIds = ref<Set<number>>(new Set())
function setBusy(id: number, on: boolean) {
  if (on) busyIds.value.add(id)
  else busyIds.value.delete(id)
  busyIds.value = new Set(busyIds.value)
}

async function withBusy<T>(id: number, fn: () => Promise<T>): Promise<[T | null, string | null]> {
  setBusy(id, true)
  const result = await projectStore.safe(fn)
  setBusy(id, false)
  return result
}

// ===== 数据加载 =====

async function loadAll() {
  await Promise.all([groupStore.fetchAll(), projectStore.fetchAll()])
}

// 加载数据并启动监控轮询（阶段 5）：挂载时 start，卸载时 stop
onMounted(async () => {
  await loadAll()
  projectStore.startPolling()
})
onBeforeUnmount(() => {
  projectStore.stopPolling()
  buildStore.reset()
})

// ===== 启停重启 =====

/** 点击卡片主体进入项目详情（日志面板） */
function openDetail(p: Project) {
  router.push({ name: 'ProjectDetail', params: { id: p.id } })
}

async function handleStart(p: Project) {
  const [, err] = await withBusy(p.id, () => projectStore.start(p.id))
  if (err) ElMessage.error(`启动失败：${err}`)
  else ElMessage.success(`「${p.name}」已启动（PID ${(projectStore.projects.find((x) => x.id === p.id)?.last_pid) ?? '-'}）`)
}

async function handleStop(p: Project) {
  const [, err] = await withBusy(p.id, () => projectStore.stop(p.id))
  if (err) ElMessage.error(`停止失败：${err}`)
  else ElMessage.success(`「${p.name}」已停止`)
}

async function handleRestart(p: Project) {
  const [, err] = await withBusy(p.id, () => projectStore.restart(p.id))
  if (err) ElMessage.error(`重启失败：${err}`)
  else ElMessage.success(`「${p.name}」已重启`)
}

// ===== 构建 / 一键发布（阶段 7） =====

/** 构建对话框显隐 + 当前构建项目 */
const buildDialogVisible = ref(false)
const buildProjectName = ref('')

/** 打开构建对话框并启动构建 */
async function handleBuild(p: Project) {
  if (!p.build_cmd?.trim()) {
    ElMessage.warning('该项目未配置构建命令')
    return
  }
  buildProjectName.value = p.name
  buildDialogVisible.value = true
  await buildStore.startBuild(p.id)
}

/**
 * 一键发布：stop → build →（构建成功则）start。
 * 编排逻辑放前端（IPC 边界原则 §4.2：Rust 只做原子操作，组合在前端）。
 */
async function handleDeploy(p: Project) {
  if (!p.build_cmd?.trim()) {
    ElMessage.warning('该项目未配置构建命令')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确定一键发布「${p.name}」吗？将执行：停止 → 构建 → 启动。`,
      '一键发布',
      { type: 'info', confirmButtonText: '发布', cancelButtonText: '取消' },
    )
  } catch {
    return // 取消
  }

  // 弹构建日志对话框（整个发布过程可见输出）
  buildProjectName.value = p.name
  buildDialogVisible.value = true

  // 1. 停止（若在运行）
  if (projectStore.isRunning(p.id)) {
    const [, stopErr] = await withBusy(p.id, () => projectStore.stop(p.id))
    if (stopErr) {
      ElMessage.error(`停止失败，已中止发布：${stopErr}`)
      return
    }
  }

  // 2. 构建（startBuild 返回的 Promise 在构建结束时 resolve）
  const buildResult = await buildStore.startBuild(p.id)
  if (!buildResult || buildResult.exit_code !== 0) {
    ElMessage.error(`构建失败（退出码 ${buildStore.exitCode}），已中止发布`)
    return
  }

  // 3. 启动
  const [, startErr] = await withBusy(p.id, () => projectStore.start(p.id))
  if (startErr) {
    ElMessage.error(`构建成功但启动失败：${startErr}`)
    return
  }
  ElMessage.success(`「${p.name}」发布完成`)
}

// ===== 删除 =====

async function handleDelete(p: Project) {
  if (projectStore.isRunning(p.id)) {
    ElMessage.warning('项目运行中，请先停止再删除')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确定删除项目「${p.name}」吗？此操作不可恢复。`,
      '删除确认',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return // 用户取消
  }
  const [, err] = await projectStore.safe(() => projectStore.remove(p.id))
  if (err) ElMessage.error(`删除失败：${err}`)
  else ElMessage.success(`已删除「${p.name}」`)
}

// ===== 新建/编辑 Dialog =====

const dialogVisible = ref(false)
const editingProject = ref<Project | null>(null)
const submitting = ref(false)

function openCreate() {
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
      ElMessage.error(`保存失败：${err}`)
      return
    }
    ElMessage.success(`已更新「${input.name}」`)
  } else {
    const [created, err] = await projectStore.safe(() => projectStore.add(input))
    submitting.value = false
    if (err || !created) {
      ElMessage.error(`创建失败：${err}`)
      return
    }
    ElMessage.success(`已创建「${created.name}」`)
  }
  dialogVisible.value = false
}
</script>

<template>
  <div class="project-list-page">
    <!-- 顶部工具栏 -->
    <div class="toolbar">
      <div class="group-tabs">
        <el-button
          v-for="tab in groupTabs"
          :key="String(tab.id)"
          :type="activeGroup === tab.id ? 'primary' : 'default'"
          size="small"
          plain
          @click="activeGroup = tab.id"
        >
          {{ tab.name }}
          <el-badge
            :value="tabCount(tab.id)"
            :max="999"
            class="tab-count"
            type="info"
          />
        </el-button>
      </div>
      <el-button type="primary" @click="openCreate">+ 新建项目</el-button>
    </div>

    <!-- 项目卡片网格 -->
    <div v-loading="projectStore.loading" class="grid">
      <template v-if="filteredProjects.length">
        <ProjectCard
          v-for="p in filteredProjects"
          :key="p.id"
          :project="p"
          :status="projectStore.statuses[p.id] ?? null"
          :busy="busyIds.has(p.id)"
          @start="handleStart(p)"
          @stop="handleStop(p)"
          @restart="handleRestart(p)"
          @build="handleBuild(p)"
          @deploy="handleDeploy(p)"
          @edit="openEdit(p)"
          @delete="handleDelete(p)"
          @open="openDetail(p)"
        />
      </template>
      <el-empty
        v-else-if="!projectStore.loading"
        description="还没有项目，点击右上角「新建项目」开始"
        class="empty-state"
      />
    </div>

    <!-- 新建/编辑对话框 -->
    <ProjectFormDialog
      v-model="dialogVisible"
      :project="editingProject"
      :groups="groupStore.groups"
      :default-group-id="activeGroup && activeGroup > 0 ? activeGroup : null"
      :submitting="submitting"
      @submit="handleSubmitForm"
    />

    <!-- 构建日志对话框（阶段 7） -->
    <BuildDialog
      v-model="buildDialogVisible"
      :project-name="buildProjectName"
    />
  </div>
</template>

<style scoped>
.project-list-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
  min-height: 0;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.group-tabs {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.tab-count {
  margin-left: 6px;
}
.tab-count :deep(.el-badge__content) {
  background-color: #909399;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 14px;
  align-content: start;
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.empty-state {
  grid-column: 1 / -1;
}
</style>
