<script setup lang="ts">
// 项目详情页 —— 阶段 6 日志中心。
//
// 布局：
//   顶部：面包屑（项目 / 项目名）+ 返回
//   主体：
//     - 项目信息卡（名称/类型/路径/启动命令/状态徽标）+ 实时指标
//     - 日志面板
//       - 实时模式：订阅 Channel，自动滚到底部，可清空当日日志
//       - 历史模式：选日期（list_log_dates），按页加载
//
// 生命周期：进入默认实时订阅；离开 stopLive + reset 释放订阅。
// 实时模式自动滚底通过 watch(lines) 实现。

import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NBreadcrumb,
  NBreadcrumbItem,
  NEmpty,
  NTag,
  NRadioGroup,
  NRadioButton,
  NSelect,
  NButton,
  useMessage,
  useDialog,
} from 'naive-ui'
import { getProject } from '@/api/project'
import { PROJECT_TYPE_LABELS, type Project } from '@/types/project'
import { useProjectStore } from '@/stores/project'
import { useLogStore } from '@/stores/log'
import StatusBadge from '@/components/project/StatusBadge.vue'
import MetricsBar from '@/components/project/MetricsBar.vue'

const route = useRoute()
const router = useRouter()
const projectStore = useProjectStore()
const logStore = useLogStore()
const message = useMessage()
const dialog = useDialog()

const projectId = computed(() => Number(route.params.id))
const project = ref<Project | null>(null)
const loadErr = ref('')

/** 日志区 DOM 引用（用于自动滚到底部） */
const logBox = ref<HTMLPreElement | null>(null)

/** 从全局轮询状态取本项目探测结果（复用列表页的轮询） */
const status = computed(() => projectStore.statuses[projectId.value] ?? null)
const health = computed(() => status.value?.health ?? 'stopped')

// ===== 初始化 =====

onMounted(async () => {
  // 取项目详情
  const [p, err] = await projectStore.safe(() => getProject(projectId.value))
  if (err || !p) {
    loadErr.value = err ?? '项目不存在'
    return
  }
  project.value = p

  // 确保轮询在运行（幂等：列表页已启动则 no-op；直接进详情页则在此启动）。
  projectStore.startPolling()

  // 默认进入实时模式
  await logStore.startLive(projectId.value)
})

onBeforeUnmount(() => {
  logStore.stopLive()
  logStore.reset()
})

// ===== 实时模式：自动滚到底部 =====

watch(
  () => logStore.lines,
  async () => {
    if (logStore.mode !== 'live') return
    await nextTick()
    const box = logBox.value
    if (box) box.scrollTop = box.scrollHeight
  },
)

// ===== 模式切换 =====

async function switchMode(m: 'live' | 'history') {
  if (m === logStore.mode) return
  if (m === 'live') {
    await logStore.startLive(projectId.value)
  } else {
    // 进入历史：先拉日期列表，默认选最新（当天）
    await logStore.fetchDates(projectId.value)
    const first = logStore.dates[0]
    if (first) {
      await logStore.loadHistory(projectId.value, first, true)
    } else {
      logStore.lines = ''
    }
  }
}

// ===== 历史翻页 =====

const historyDateOptions = computed(() =>
  logStore.dates.map((d) => ({
    label: `${d.slice(0, 4)}-${d.slice(4, 6)}-${d.slice(6, 8)}`,
    value: d,
  })),
)

async function pickHistoryDate(date: string) {
  if (!date) return
  await logStore.loadHistory(projectId.value, date, true)
}

async function nextPage() {
  await logStore.loadNextPage(projectId.value)
}

// ===== 清空日志 =====

async function handleClear() {
  dialog.warning({
    title: '清空确认',
    content: '确定清空当天日志吗？此操作不可恢复。',
    positiveText: '清空',
    negativeText: '取消',
    onPositiveClick: async () => {
      const ok = await logStore.clear(projectId.value)
      if (ok) message.success('已清空当日日志')
    },
  })
}

function goBack() {
  router.back()
}

/** 行数统计（仅展示用） */
const lineCount = computed(() => {
  const t = logStore.lines
  if (!t) return 0
  let n = 1
  for (let i = 0; i < t.length; i++) {
    if (t.charCodeAt(i) === 10) n++
  }
  return n
})
</script>

<template>
  <div class="detail-page">
    <!-- 面包屑 + 返回 -->
    <div class="topbar">
      <NBreadcrumb>
        <NBreadcrumbItem clickable @click="goBack">项目</NBreadcrumbItem>
        <NBreadcrumbItem>{{ project?.name ?? '...' }}</NBreadcrumbItem>
      </NBreadcrumb>
    </div>

    <!-- 加载错误 -->
    <NEmpty v-if="loadErr" :description="loadErr" />

    <template v-else-if="project">
      <!-- 项目信息卡 -->
      <div class="info-card card-surface">
        <div class="info-head">
          <span class="name">{{ project.name }}</span>
          <NTag size="small" type="primary" :bordered="false">
            {{ PROJECT_TYPE_LABELS[project.type] }}
          </NTag>
          <StatusBadge :health="health" />
        </div>
        <div class="info-meta">
          <div class="meta-row" :title="project.path">
            <span class="lbl">路径</span>
            <span class="val code">{{ project.path || '-' }}</span>
          </div>
          <div class="meta-row" :title="project.start_cmd">
            <span class="lbl">启动</span>
            <span class="val code">{{ project.start_cmd || '-' }}</span>
          </div>
          <div class="meta-row">
            <span class="lbl">端口</span>
            <span class="val">{{ project.expected_ports.length ? project.expected_ports.join(' / ') : '-' }}</span>
          </div>
        </div>
        <!-- 运行中显示实时指标 -->
        <div v-if="status && health !== 'stopped'" class="metrics-wrap">
          <MetricsBar :status="status" />
        </div>
      </div>

      <!-- 日志面板 -->
      <div class="log-card card-surface">
        <div class="log-header">
          <NRadioGroup
            :value="logStore.mode"
            size="small"
            @update:value="(v: string) => switchMode(v as 'live' | 'history')"
          >
            <NRadioButton value="live">实时</NRadioButton>
            <NRadioButton value="history">历史</NRadioButton>
          </NRadioGroup>

          <!-- 历史模式：日期选择 + 翻页 -->
          <div v-if="logStore.mode === 'history'" class="history-controls">
            <NSelect
              :value="logStore.historyDate"
              :options="historyDateOptions"
              size="small"
              placeholder="选择日期"
              style="width: 150px"
              @update:value="(v: string) => pickHistoryDate(v)"
            />
            <NButton
              size="small"
              :disabled="!logStore.historyHasMore"
              @click="nextPage"
            >
              加载更多
            </NButton>
          </div>

          <div class="log-actions">
            <span class="line-count">{{ lineCount }} 行</span>
            <NButton
              v-if="logStore.mode === 'live'"
              size="small"
              tertiary
              @click="handleClear"
            >
              清空
            </NButton>
          </div>
        </div>

        <div class="log-body">
          <pre ref="logBox" class="log-text">{{ logStore.lines || '（暂无日志）' }}</pre>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.detail-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}
.topbar {
  flex-shrink: 0;
}

/* 信息卡 */
.info-card {
  padding: 14px 18px;
  flex-shrink: 0;
}
.info-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}
.info-head .name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}
.info-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--text-secondary);
}
.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.lbl {
  width: 32px;
  color: var(--text-tertiary);
  flex-shrink: 0;
}
.val {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.val.code {
  font-family: var(--code-font);
  color: var(--text-primary);
}
.metrics-wrap {
  margin-top: 10px;
  padding: 8px 10px;
  background: var(--status-running-soft);
  border-radius: 4px;
}

/* 日志面板（占满剩余高度） */
.log-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.log-header {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  padding: 8px 12px;
  border-bottom: 1px solid var(--divider);
  background: var(--toolbar-bg);
  flex-shrink: 0;
}
.history-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}
.log-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 10px;
}
.line-count {
  font-size: 12px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
}
.log-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
}
.log-text {
  flex: 1;
  margin: 0;
  padding: 12px 14px;
  font-family: var(--code-font);
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--terminal-info-fg);
  background: var(--terminal-info-bg);
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
