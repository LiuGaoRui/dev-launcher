<script setup lang="ts">
// 内存清理抽屉 —— 全系统开发进程扫描 + 智能推荐 + 杀进程/回收内存。
//
// 布局（NDrawer, 右侧 640px）：
//   顶部：系统内存概览条 + 推荐清理摘要
//   工具栏：一键回收全部 / 全选推荐 / 一键清理推荐 / 刷新 / 搜索框
//   进程列表：复选框 + 类型图标 + 项目提示/命令行 + 内存(整树) + 分类标签 + 行内「回收」/「杀」
//   底部：已选统计 + 回收选中 + 清理选中
//
// 两个正交动作：
//   - 杀（trash）：终止进程树，进程消失——用于孤立残留、泄漏的 dev server
//   - 回收（water）：修剪工作集换出冷页，进程继续跑——用于 IDE 内存回收
// IDE 进程不可杀（受保护），但可回收。

import { computed, onBeforeUnmount, ref, watch } from 'vue'
import {
  NDrawer,
  NDrawerContent,
  NButton,
  NIcon,
  NInput,
  NTag,
  NSpin,
  NEmpty,
  useMessage,
  useDialog,
} from 'naive-ui'
import {
  TrashOutline,
  RefreshOutline,
  SearchOutline,
  ServerOutline,
  CloseCircle,
  LayersOutline,
  WaterOutline,
  LockClosedOutline,
  LockOpenOutline,
} from '@vicons/ionicons5'
import { useCleanerStore } from '@/stores/cleaner'
import { CATEGORY_META, type DevProcInfo } from '@/types/cleaner'
import { formatBytes } from '@/types/monitor'

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ 'update:show': [value: boolean] }>()

const cleanerStore = useCleanerStore()
const message = useMessage()
const dialog = useDialog()

/** 搜索关键字（过滤项目提示/命令行/进程名） */
const keyword = ref('')

/** 过滤后的进程列表 */
const filteredProcesses = computed<DevProcInfo[]>(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return cleanerStore.processes
  return cleanerStore.processes.filter((p) => {
    return (
      p.display_title.toLowerCase().includes(kw) ||
      p.cmdline_hint.toLowerCase().includes(kw) ||
      p.project_path.toLowerCase().includes(kw) ||
      p.cmdline_summary.toLowerCase().includes(kw) ||
      p.main_script.toLowerCase().includes(kw) ||
      (p.cwd ?? '').toLowerCase().includes(kw) ||
      p.exe.toLowerCase().includes(kw) ||
      p.cmdline.toLowerCase().includes(kw) ||
      p.name.toLowerCase().includes(kw) ||
      String(p.pid) === kw
    )
  })
})

/** 内存使用率 → 颜色档位 */
function memColor(percent: number): string {
  if (percent >= 85) return '#f5222d'
  if (percent >= 70) return '#faad14'
  return '#52c41a'
}

/** 获取某分类的标签元数据 */
function catMeta(p: DevProcInfo) {
  return CATEGORY_META[p.category]
}

/** 进程类型图标（java ☕ / node 🟢） */
function kindIcon(kind: string): string {
  if (kind === 'java') return '☕'
  return '🟢'
}

// ===== 抽屉开关 + 轮询生命周期 =====

watch(
  () => props.show,
  (open) => {
    if (open) {
      cleanerStore.startPolling()
    } else {
      cleanerStore.stopPolling()
    }
  },
)

onBeforeUnmount(() => {
  cleanerStore.stopPolling()
})

// ===== 清理操作 =====

/** 一键回收全部开发进程的内存（修剪工作集，不杀进程，含 IDE） */
async function handleTrimAll() {
  if (cleanerStore.processes.length === 0) {
    message.info('暂无可回收的进程')
    return
  }
  const count = cleanerStore.processes.length
  const totalMem = cleanerStore.totalProcessMemory
  dialog.info({
    title: '一键回收内存',
    content: `将对 ${count} 个开发进程（含 IDE）修剪工作集，当前合计占用 ${formatBytes(totalMem)}。进程不会终止，只是换出不活跃内存页。`,
    positiveText: '回收',
    negativeText: '取消',
    onPositiveClick: async () => {
      const [result, err] = await cleanerStore.trimAll()
      if (err) {
        message.error(`回收失败：${err}`)
        return
      }
      if (result) {
        const freed = formatBytes(result.freed_bytes)
        if (result.failed > 0) {
          message.warning(`已回收 ${result.trimmed} 项（释放 ${freed}），${result.failed} 项失败`)
        } else {
          message.success(`已回收 ${result.trimmed} 项，释放 ${freed} 内存`)
        }
      }
    },
  })
}

/** 回收已选中进程的内存 */
async function handleTrimSelected() {
  const selectedCount = cleanerStore.selectedPidTrees.length
  if (selectedCount === 0) {
    message.warning('请先选择要回收的进程')
    return
  }
  const [result, err] = await cleanerStore.trimSelected()
  if (err) {
    message.error(`回收失败：${err}`)
    return
  }
  if (result) {
    const freed = formatBytes(result.freed_bytes)
    message.success(`已回收 ${result.trimmed} 项，释放 ${freed} 内存`)
  }
}

/** 回收单个进程的内存（行内「回收」按钮） */
async function handleTrimOne(p: DevProcInfo) {
  const [result, err] = await cleanerStore.trimOne(p)
  if (err) {
    message.error(`回收失败：${err}`)
    return
  }
  if (result) {
    const freed = formatBytes(result.freed_bytes)
    message.success(`已回收，释放 ${freed} 内存`)
  }
}

async function handleKillSelected() {
  if (cleanerStore.selectedPidTrees.length === 0) {
    message.warning('请先选择要清理的进程')
    return
  }
  const count = cleanerStore.selectedPidTrees.length
  const mem = cleanerStore.selectedMemory
  dialog.warning({
    title: '确认清理',
    content: `将强制终止 ${count} 个进程树，预计释放 ${formatBytes(mem)} 内存。此操作不可恢复，请确认未误选正在调试的进程。`,
    positiveText: '清理',
    negativeText: '取消',
    onPositiveClick: async () => {
      const [result, err] = await cleanerStore.killSelected()
      if (err) {
        message.error(`清理失败：${err}`)
        return
      }
      if (result) {
        const freed = formatBytes(result.freed_bytes)
        if (result.failed > 0) {
          message.warning(
            `已清理 ${result.killed} 项（释放 ${freed}），${result.failed} 项失败（可能权限不足）`,
          )
        } else {
          message.success(`已清理 ${result.killed} 项，释放 ${freed} 内存`)
        }
      }
    },
  })
}

async function handleKillOne(p: DevProcInfo) {
  dialog.warning({
    title: '确认终止',
    content: `将强制终止「${p.cmdline_hint || p.name}」（PID ${p.pid}，占用 ${formatBytes(p.tree_memory_bytes)}）。`,
    positiveText: '终止',
    negativeText: '取消',
    onPositiveClick: async () => {
      const [result, err] = await cleanerStore.killOne(p)
      if (err) {
        message.error(`终止失败：${err}`)
        return
      }
      if (result) {
        const freed = formatBytes(result.freed_bytes)
        message.success(`已终止，释放 ${freed} 内存`)
      }
    },
  })
}

/** 一键清理推荐项：全选推荐后触发清理 */
async function handleKillRecommended() {
  cleanerStore.selectAllRecommended()
  // 若推荐项已全部选中（再次点击 = 取消），不执行清理
  if (cleanerStore.selectedPidTrees.length === 0) {
    message.info('已取消选中推荐项')
    return
  }
  await handleKillSelected()
}

/** 统一解锁：解除所有锁定进程（带二次确认） */
async function handleUnlockAll() {
  const n = cleanerStore.lockedCount
  if (n === 0) return
  dialog.info({
    title: '解锁全部',
    content: `将解除全部 ${n} 个进程的锁定状态，解锁后可被选中、清理和回收内存。`,
    positiveText: '解锁',
    negativeText: '取消',
    onPositiveClick: () => {
      cleanerStore.unlockAll()
      message.success(`已解锁 ${n} 个进程`)
    },
  })
}

/** 切换 IDE 保护（无二次确认，底部按钮直接切换） */
function handleToggleIdeProtection() {
  cleanerStore.toggleAllowKillIde()
}
</script>

<template>
  <NDrawer
    :show="props.show"
    :width="640"
    placement="right"
    :auto-focus="false"
    @update:show="emit('update:show', $event)"
  >
    <NDrawerContent :native-scrollbar="false" closable>
      <template #header>
        <div class="drawer-header">
          <NIcon size="16"><ServerOutline /></NIcon>
          <span>内存清理</span>
          <span class="proc-count">{{ cleanerStore.processes.length }} 个进程</span>
        </div>
      </template>

      <!-- 顶部：系统内存概览 -->
      <div class="mem-overview">
        <div class="mem-bar-row">
          <span class="mem-label">系统内存</span>
          <div class="mem-bar-track">
            <div
              class="mem-bar-fill"
              :style="{
                width: `${cleanerStore.systemMem?.used_percent ?? 0}%`,
                background: memColor(cleanerStore.systemMem?.used_percent ?? 0),
              }"
            />
          </div>
          <span class="mem-text">
            {{ formatBytes(cleanerStore.systemMem?.used_bytes ?? 0) }}
            / {{ formatBytes(cleanerStore.systemMem?.total_bytes ?? 0) }}
          </span>
        </div>
        <div class="recommend-summary" v-if="cleanerStore.recommendedPids.length">
          <span class="recommend-dot" />
          推荐清理
          <strong>{{ cleanerStore.recommendedPids.length }}</strong>
          项，预计释放
          <strong class="freed">{{ formatBytes(cleanerStore.totalRecommendedMemory) }}</strong>
        </div>
      </div>

      <!-- 工具栏 -->
      <div class="toolbar">
        <div class="toolbar-left">
          <NButton
            size="small"
            type="primary"
            secondary
            :disabled="cleanerStore.processes.length === 0 || cleanerStore.trimming"
            :loading="cleanerStore.trimming"
            @click="handleTrimAll"
          >
            <template #icon><NIcon><WaterOutline /></NIcon></template>
            一键回收全部
          </NButton>
          <NButton
            size="small"
            type="warning"
            secondary
            :disabled="cleanerStore.recommendedPids.length === 0 || cleanerStore.killing"
            @click="handleKillRecommended"
          >
            <template #icon><NIcon><TrashOutline /></NIcon></template>
            一键清理推荐
          </NButton>
          <NButton
            size="small"
            secondary
            :disabled="cleanerStore.recommendedPids.length === 0"
            @click="cleanerStore.selectAllRecommended()"
          >
            {{ cleanerStore.recommendedPids.length > 0 && cleanerStore.recommendedPids.every((pid: number) => cleanerStore.selectedPids.has(pid)) ? '取消推荐' : '全选推荐' }}
          </NButton>
          <NButton
            v-if="cleanerStore.lockedCount > 0"
            size="small"
            quaternary
            :title="`当前有 ${cleanerStore.lockedCount} 个锁定进程`"
            @click="handleUnlockAll"
          >
            <template #icon><NIcon><LockOpenOutline /></NIcon></template>
            解锁全部({{ cleanerStore.lockedCount }})
          </NButton>
          <NButton size="small" quaternary :loading="cleanerStore.scanning" @click="cleanerStore.scan()">
            <template #icon><NIcon><RefreshOutline /></NIcon></template>
          </NButton>
        </div>
        <NInput
          v-model:value="keyword"
          size="small"
          placeholder="搜索名称 / 路径 / PID"
          clearable
          style="width: 200px"
        >
          <template #prefix><NIcon :component="SearchOutline" /></template>
        </NInput>
      </div>

      <!-- 进程列表 -->
      <div class="proc-list">
        <div v-if="cleanerStore.scanning && cleanerStore.processes.length === 0" class="list-loading">
          <NSpin size="small" />
        </div>

        <div v-else-if="filteredProcesses.length === 0" class="list-empty">
          <NEmpty :description="keyword ? '无匹配进程' : '未发现开发进程'" size="small" />
        </div>

        <div v-else class="proc-rows">
          <div
            v-for="p in filteredProcesses"
            :key="p.pid"
            class="proc-row"
            :class="{
              'proc-row--recommended': p.recommended,
              'proc-row--protected': cleanerStore.isProtected(p),
              'proc-row--locked': cleanerStore.isLocked(p),
              'proc-row--selected': cleanerStore.selectedPids.has(p.pid),
              'proc-row--clickable': cleanerStore.isActionable(p),
            }"
            @click="cleanerStore.toggleSelect(p.pid)"
          >
            <!-- 复选框（锁定/保护 时禁用） -->
            <label class="proc-check" :class="{ 'proc-check--disabled': !cleanerStore.isActionable(p) }">
              <input
                type="checkbox"
                :checked="cleanerStore.selectedPids.has(p.pid)"
                :disabled="!cleanerStore.isActionable(p)"
                @change="cleanerStore.toggleSelect(p.pid)"
              />
            </label>

            <!-- 类型图标 -->
            <span class="proc-kind">{{ kindIcon(p.kind) }}</span>

            <!-- 展示名 + 项目路径 + 命令摘要 + 工作目录 -->
            <div class="proc-main">
              <div class="proc-title-row">
                <span class="proc-title" :title="p.display_title">{{ p.display_title || p.name }}</span>
                <NTag
                  size="tiny"
                  round
                  :bordered="false"
                  :color="{ color: catMeta(p).color, textColor: '#fff' }"
                >
                  {{ catMeta(p).label }}
                </NTag>
                <span v-if="cleanerStore.isLocked(p)" class="lock-badge">已锁定</span>
                <span v-if="p.recommended" class="rec-badge">推荐</span>
              </div>
              <!-- 项目路径 -->
              <div v-if="p.project_path" class="proc-path" :title="`项目路径: ${p.project_path}`">
                <span class="proc-path-prefix">📁</span>{{ p.project_path }}
              </div>
              <!-- 命令行摘要（正在执行什么） -->
              <div v-if="p.cmdline_summary" class="proc-summary" :title="`完整命令行: ${p.cmdline}`">
                <span class="proc-summary-prefix">💻</span>{{ p.cmdline_summary }}
              </div>
              <!-- 工作目录 + PID -->
              <div class="proc-meta">
                <span v-if="p.cwd" class="proc-cwd" :title="`工作目录: ${p.cwd}`">
                  📂 {{ p.cwd }}
                </span>
                <span class="proc-pid">PID {{ p.pid }}</span>
              </div>
            </div>

            <!-- 内存 -->
            <div class="proc-mem">
              <span class="proc-mem-val">{{ formatBytes(p.tree_memory_bytes) }}</span>
              <span class="proc-mem-cpu">CPU {{ p.cpu_percent.toFixed(1) }}%</span>
            </div>

            <!-- 单独回收按钮（锁定时禁用） -->
            <button
              class="proc-action-btn proc-trim-btn"
              :disabled="cleanerStore.isLocked(p) || cleanerStore.trimming"
              :title="cleanerStore.isLocked(p) ? '已锁定，不可回收' : '回收内存（修剪工作集，不终止进程）'"
              @click.stop="handleTrimOne(p)"
            >
              <NIcon size="14"><WaterOutline /></NIcon>
            </button>

            <!-- 单独杀按钮（锁定/IDE 保护禁用） -->
            <button
              class="proc-action-btn proc-kill-btn"
              :disabled="!cleanerStore.isActionable(p) || cleanerStore.killing"
              :title="cleanerStore.isLocked(p) ? '已锁定，不可终止' : cleanerStore.isProtected(p) ? '受保护的 IDE 进程不可终止' : '终止此进程树'"
              @click.stop="handleKillOne(p)"
            >
              <NIcon size="14"><CloseCircle /></NIcon>
            </button>

            <!-- 锁定/解锁按钮 -->
            <button
              class="proc-action-btn proc-lock-btn"
              :class="{ 'proc-lock-btn--locked': cleanerStore.isLocked(p) }"
              :title="cleanerStore.isLocked(p) ? '点击解锁' : '锁定（锁定后不可清理和回收）'"
              @click.stop="cleanerStore.toggleLock(p.pid)"
            >
              <NIcon size="14">
                <LockClosedOutline v-if="cleanerStore.isLocked(p)" />
                <LockOpenOutline v-else />
              </NIcon>
            </button>
          </div>
        </div>
      </div>

      <!-- 底部：已选统计 + 清理 -->
      <template #footer>
        <div class="drawer-footer">
          <div class="footer-stat">
            <NIcon size="14"><LayersOutline /></NIcon>
            <span>已选 <strong>{{ cleanerStore.selectedPidTrees.length }}</strong> 项</span>
            <span class="footer-mem">{{ formatBytes(cleanerStore.selectedMemory) }}</span>
          </div>
          <div class="footer-actions">
            <NButton
              size="small"
              quaternary
              :class="{ 'ide-toggle-btn--unlocked': cleanerStore.allowKillIde }"
              @click="handleToggleIdeProtection"
            >
              <template #icon>
                <NIcon>
                  <LockOpenOutline v-if="cleanerStore.allowKillIde" />
                  <LockClosedOutline v-else />
                </NIcon>
              </template>
              {{ cleanerStore.allowKillIde ? '锁定IDE' : '解锁IDE' }}
            </NButton>
            <NButton size="small" quaternary :disabled="cleanerStore.selectedPidTrees.length === 0" @click="cleanerStore.clearSelection()">
              清空选择
            </NButton>
            <NButton
              size="small"
              type="primary"
              secondary
              :disabled="cleanerStore.selectedPidTrees.length === 0 || cleanerStore.trimming"
              :loading="cleanerStore.trimming"
              @click="handleTrimSelected"
            >
              <template #icon><NIcon><WaterOutline /></NIcon></template>
              回收选中
            </NButton>
            <NButton
              size="small"
              type="error"
              :disabled="cleanerStore.selectedPidTrees.length === 0 || cleanerStore.killing"
              :loading="cleanerStore.killing"
              @click="handleKillSelected"
            >
              <template #icon><NIcon><TrashOutline /></NIcon></template>
              清理选中
            </NButton>
          </div>
        </div>
      </template>
    </NDrawerContent>
  </NDrawer>
</template>

<style scoped>
.drawer-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  font-size: 14px;
}
.proc-count {
  font-size: 11.5px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
  font-weight: 400;
  margin-left: 4px;
}

/* 顶部内存概览 */
.mem-overview {
  padding: 0 4px 12px;
  border-bottom: 1px solid var(--divider);
  margin-bottom: 10px;
}
.mem-bar-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.mem-label {
  font-size: 12px;
  color: var(--text-secondary);
  flex-shrink: 0;
  width: 56px;
}
.mem-bar-track {
  flex: 1;
  height: 8px;
  background: var(--card-bg);
  border: 1px solid var(--divider);
  border-radius: 4px;
  overflow: hidden;
}
.mem-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease, background 0.4s ease;
}
.mem-text {
  font-size: 11.5px;
  font-family: var(--code-font);
  color: var(--text-secondary);
  flex-shrink: 0;
}
.recommend-summary {
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: 4px;
}
.recommend-summary strong {
  color: var(--text-primary);
  margin: 0 2px;
}
.recommend-summary .freed {
  color: var(--status-running);
}
.recommend-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--status-warning);
  margin-right: 4px;
}

/* 底部 IDE 保护切换：解锁态红色高亮提醒 */
.ide-toggle-btn--unlocked {
  color: #f5222d !important;
}

/* 工具栏 */
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.toolbar-left {
  display: flex;
  gap: 6px;
  align-items: center;
}

/* 进程列表 */
.proc-list {
  min-height: 120px;
}
.list-loading,
.list-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
}
.proc-rows {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* 单行 */
.proc-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 5px;
  border: 1px solid transparent;
  transition: background 0.12s, border-color 0.12s;
}
.proc-row:hover {
  background: var(--card-hover-bg);
}
.proc-row--selected {
  background: var(--card-hover-bg);
  border-color: var(--accent);
}
.proc-row--recommended {
  border-left: 3px solid var(--status-warning);
}
.proc-row--protected {
  opacity: 0.6;
}
/* 可操作行：点击即切换选中 */
.proc-row--clickable {
  cursor: pointer;
}

/* 复选框 */
.proc-check {
  display: flex;
  align-items: center;
  cursor: pointer;
  flex-shrink: 0;
  width: 16px;
}
.proc-check--disabled {
  cursor: not-allowed;
}
.proc-check input {
  margin: 0;
  cursor: inherit;
}

/* 类型图标 */
.proc-kind {
  font-size: 14px;
  flex-shrink: 0;
  width: 18px;
  text-align: center;
}

/* 主体信息 */
.proc-main {
  flex: 1;
  min-width: 0;
}
.proc-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.proc-title {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 220px;
}
.proc-path {
  font-size: 10.5px;
  color: var(--text-secondary);
  font-family: var(--code-font);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
  display: flex;
  align-items: center;
  gap: 3px;
}
.proc-path-prefix {
  flex-shrink: 0;
  opacity: 0.7;
}
.proc-summary {
  font-size: 10.5px;
  color: var(--text-secondary);
  font-family: var(--code-font);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 1px;
  display: flex;
  align-items: center;
  gap: 3px;
}
.proc-summary-prefix {
  flex-shrink: 0;
  opacity: 0.7;
}
.proc-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 1px;
}
.proc-cwd {
  font-size: 9.5px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 240px;
}
.proc-pid {
  font-size: 9.5px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
  flex-shrink: 0;
}
.rec-badge {
  font-size: 9.5px;
  background: var(--status-warning);
  color: #fff;
  padding: 0 5px;
  border-radius: 3px;
  line-height: 14px;
  flex-shrink: 0;
}

/* 内存列 */
.proc-mem {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 1px;
  flex-shrink: 0;
  width: 80px;
}
.proc-mem-val {
  font-size: 12px;
  font-weight: 600;
  font-family: var(--code-font);
  color: var(--text-primary);
}
.proc-mem-cpu {
  font-size: 10px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
}

/* 操作按钮基类（回收/杀/锁定 共享） */
.proc-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  flex-shrink: 0;
  transition: background 0.12s, color 0.12s;
}
.proc-action-btn:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

/* 回收按钮（修剪工作集，蓝色系） */
.proc-trim-btn:hover:not(:disabled) {
  background: var(--status-running-soft);
  color: var(--accent);
}

/* 杀按钮（红色系） */
.proc-kill-btn:hover:not(:disabled) {
  background: var(--status-warning-soft);
  color: var(--status-warning);
}

/* 锁定按钮 */
.proc-lock-btn:hover {
  background: var(--card-hover-bg);
  color: var(--text-secondary);
}
/* 已锁定状态：图标高亮 */
.proc-lock-btn--locked {
  color: #722ed1;
}
.proc-lock-btn--locked:hover {
  background: rgba(114, 46, 209, 0.12);
  color: #722ed1;
}

/* 锁定行视觉区分 */
.proc-row--locked {
  border-left: 3px solid #722ed1;
}
/* 锁定角标 */
.lock-badge {
  font-size: 9.5px;
  background: #722ed1;
  color: #fff;
  padding: 0 5px;
  border-radius: 3px;
  line-height: 14px;
  flex-shrink: 0;
}

/* 底部 */
.drawer-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}
.footer-stat {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
.footer-stat strong {
  color: var(--text-primary);
  font-family: var(--code-font);
}
.footer-mem {
  font-family: var(--code-font);
  color: var(--status-running);
  font-weight: 600;
}
.footer-actions {
  display: flex;
  gap: 8px;
}
</style>
