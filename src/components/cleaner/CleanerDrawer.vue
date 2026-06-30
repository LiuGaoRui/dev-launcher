<script setup lang="ts">
// 内存清理抽屉 —— 全系统开发进程扫描 + 智能推荐 + 批量/单独清理。
//
// 布局（NDrawer, 右侧 640px）：
//   顶部：系统内存概览条 + 推荐清理摘要
//   工具栏：全选推荐 / 一键清理推荐 / 刷新 / 搜索框
//   进程列表：复选框 + 类型图标 + 项目提示/命令行 + 内存(整树) + 分类标签 + 单独「杀」
//   底部：已选统计 + 清理按钮
//
// 智能推荐：孤立进程、高内存构建 daemon / dev server（后端 dev_scan 判定）。
// 安全：IDE 进程受保护不可选中、不可清理；所有清理需用户点击确认。

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
      p.cmdline_hint.toLowerCase().includes(kw) ||
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
          <NButton size="small" quaternary :loading="cleanerStore.scanning" @click="cleanerStore.scan()">
            <template #icon><NIcon><RefreshOutline /></NIcon></template>
          </NButton>
        </div>
        <NInput
          v-model:value="keyword"
          size="small"
          placeholder="搜索进程 / 命令行 / PID"
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
              'proc-row--selected': cleanerStore.selectedPids.has(p.pid),
            }"
          >
            <!-- 复选框 -->
            <label class="proc-check" :class="{ 'proc-check--disabled': cleanerStore.isProtected(p) }">
              <input
                type="checkbox"
                :checked="cleanerStore.selectedPids.has(p.pid)"
                :disabled="cleanerStore.isProtected(p)"
                @change="cleanerStore.toggleSelect(p.pid)"
              />
            </label>

            <!-- 类型图标 -->
            <span class="proc-kind">{{ kindIcon(p.kind) }}</span>

            <!-- 项目提示 + 命令行 -->
            <div class="proc-main">
              <div class="proc-hint-row">
                <span class="proc-hint" :title="p.cmdline_hint">{{ p.cmdline_hint || p.name }}</span>
                <NTag
                  size="tiny"
                  round
                  :bordered="false"
                  :color="{ color: catMeta(p).color, textColor: '#fff' }"
                >
                  {{ catMeta(p).label }}
                </NTag>
                <span v-if="p.recommended" class="rec-badge">推荐</span>
              </div>
              <div class="proc-cmdline" :title="p.cmdline">{{ p.cmdline }}</div>
            </div>

            <!-- 内存 -->
            <div class="proc-mem">
              <span class="proc-mem-val">{{ formatBytes(p.tree_memory_bytes) }}</span>
              <span class="proc-mem-cpu">CPU {{ p.cpu_percent.toFixed(1) }}%</span>
            </div>

            <!-- 单独杀按钮 -->
            <button
              class="proc-kill-btn"
              :disabled="cleanerStore.isProtected(p) || cleanerStore.killing"
              :title="cleanerStore.isProtected(p) ? '受保护的 IDE 进程' : '终止此进程树'"
              @click.stop="handleKillOne(p)"
            >
              <NIcon size="14"><CloseCircle /></NIcon>
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
            <NButton size="small" quaternary :disabled="cleanerStore.selectedPidTrees.length === 0" @click="cleanerStore.clearSelection()">
              清空选择
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
.proc-hint-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.proc-hint {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 180px;
}
.proc-cmdline {
  font-size: 10.5px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
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

/* 单独杀按钮 */
.proc-kill-btn {
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
.proc-kill-btn:hover:not(:disabled) {
  background: var(--status-warning-soft);
  color: var(--status-warning);
}
.proc-kill-btn:disabled {
  cursor: not-allowed;
  opacity: 0.4;
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
