<script setup lang="ts">
// 项目详情页 —— 日志中心。
//
// 布局：
//   顶部：面包屑（项目 / 项目名）+ 返回
//   主体：
//     - 项目信息卡（名称/类型/路径/启动命令/状态徽标）+ 实时指标
//     - 日志面板（双 Tab：启动日志 / 构建日志）
//       - 实时订阅对应日志文件（start.log / build.log），自动滚到底部，可清空
//
// 生命周期：进入默认订阅启动日志；离开 stopLive + reset 释放订阅。
// 自动滚底通过 watch(lines) 实现。

import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  NBreadcrumb,
  NBreadcrumbItem,
  NEmpty,
  NTag,
  NRadioButton,
  NRadioGroup,
  NButton,
  useDialog,
} from 'naive-ui'
import { getProject } from '@/api/project'
import { PROJECT_TYPE_LABELS, type Project } from '@/types/project'
import { useProjectStore } from '@/stores/project'
import { useLogStore } from '@/stores/log'
import type { LogType } from '@/types/log'
import StatusBadge from '@/components/project/StatusBadge.vue'
import MetricsBar from '@/components/project/MetricsBar.vue'

const route = useRoute()
const router = useRouter()
const projectStore = useProjectStore()
const logStore = useLogStore()
const dialog = useDialog()

const projectId = computed(() => Number(route.params.id))
const project = ref<Project | null>(null)
const loadErr = ref('')

/** 日志区 DOM 引用（用于自动滚到底部） */
const logBox = ref<HTMLPreElement | null>(null)

/** 当前日志类型 Tab（start/build） */
const logType = ref<LogType>('start')

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

  // 默认订阅启动日志
  await logStore.startLive(projectId.value, logType.value)
})

onBeforeUnmount(() => {
  // reset 内部已 stopLive（作废 token + 后端退订）
  logStore.reset()
})

// ===== 自动滚到底部 =====

watch(
  () => logStore.lines,
  async () => {
    await nextTick()
    const box = logBox.value
    if (box) box.scrollTop = box.scrollHeight
  },
)

// ===== 切换日志类型 Tab =====

async function switchLogType(t: LogType) {
  if (t === logType.value) return
  logType.value = t
  await logStore.startLive(projectId.value, t)
}

// ===== 清空当前类型日志 =====

async function handleClear() {
  dialog.warning({
    title: '清空确认',
    content: `确定清空${logType.value === 'start' ? '启动' : '构建'}日志吗？此操作不可恢复。`,
    positiveText: '清空',
    negativeText: '取消',
    onPositiveClick: async () => {
      await logStore.clear(projectId.value, logType.value)
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
            :value="logType"
            size="small"
            @update:value="(v: string) => switchLogType(v as LogType)"
          >
            <NRadioButton value="start">启动日志</NRadioButton>
            <NRadioButton value="build">构建日志</NRadioButton>
          </NRadioGroup>

          <div class="log-actions">
            <span class="line-count">{{ lineCount }} 行</span>
            <NButton size="small" tertiary @click="handleClear">
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
