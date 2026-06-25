<script setup lang="ts">
// 项目详情页 —— 阶段 6 日志中心。
//
// 布局：
//   顶部：项目信息卡（名称/类型/路径/启动命令/状态徽标）+ 返回按钮
//   主体：日志面板
//     - 实时模式：订阅 Channel，自动滚到底部，可清空当日日志
//     - 历史模式：选日期（list_log_dates），按页加载（上/下一页）
//
// 生命周期：进入默认实时订阅；离开 stopLive + reset 释放订阅。
// 实时模式自动滚底通过 watch(lines) 实现。

import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, Delete } from '@element-plus/icons-vue'
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

  // 列页若没在轮询则启动（详情页复用同一轮询拿本项目状态）
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

async function pickHistoryDate(date: string) {
  if (!date) return
  await logStore.loadHistory(projectId.value, date, true)
}

async function nextPage() {
  await logStore.loadNextPage(projectId.value)
}

// ===== 清空日志 =====

async function handleClear() {
  try {
    await ElMessageBox.confirm('确定清空当天日志吗？此操作不可恢复。', '清空确认', {
      type: 'warning',
      confirmButtonText: '清空',
      cancelButtonText: '取消',
    })
  } catch {
    return // 取消
  }
  const ok = await logStore.clear(projectId.value)
  if (ok) ElMessage.success('已清空当日日志')
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
    <!-- 顶部工具栏 -->
    <div class="topbar">
      <el-button :icon="ArrowLeft" text @click="goBack">返回</el-button>
    </div>

    <!-- 加载错误 -->
    <el-empty v-if="loadErr" :description="loadErr" />

    <template v-else-if="project">
      <!-- 项目信息卡 -->
      <el-card class="info-card" shadow="never" :body-style="{ padding: '16px 20px' }">
        <div class="info-head">
          <span class="name">{{ project.name }}</span>
          <el-tag size="small" type="primary" effect="plain">
            {{ PROJECT_TYPE_LABELS[project.type] }}
          </el-tag>
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
      </el-card>

      <!-- 日志面板 -->
      <el-card class="log-card" shadow="never">
        <template #header>
          <div class="log-header">
            <el-radio-group
              :model-value="logStore.mode"
              size="small"
              @change="(v: string | number | boolean) => switchMode(v as 'live' | 'history')"
            >
              <el-radio-button value="live">实时</el-radio-button>
              <el-radio-button value="history">历史</el-radio-button>
            </el-radio-group>

            <!-- 历史模式：日期选择 + 翻页 -->
            <div v-if="logStore.mode === 'history'" class="history-controls">
              <el-select
                :model-value="logStore.historyDate"
                size="small"
                placeholder="选择日期"
                style="width: 150px"
                @change="(v: string) => pickHistoryDate(v)"
              >
                <el-option
                  v-for="d in logStore.dates"
                  :key="d"
                  :label="`${d.slice(0,4)}-${d.slice(4,6)}-${d.slice(6,8)}`"
                  :value="d"
                />
              </el-select>
              <el-button
                size="small"
                :disabled="!logStore.historyHasMore"
                @click="nextPage"
              >
                加载更多
              </el-button>
            </div>

            <div class="log-actions">
              <span class="line-count">{{ lineCount }} 行</span>
              <el-button
                v-if="logStore.mode === 'live'"
                size="small"
                :icon="Delete"
                @click="handleClear"
              >
                清空
              </el-button>
            </div>
          </div>
        </template>

        <div v-loading="logStore.loading" class="log-body">
          <pre ref="logBox" class="log-text">{{ logStore.lines || '（暂无日志）' }}</pre>
        </div>
      </el-card>
    </template>
  </div>
</template>

<style scoped>
.detail-page {
  display: flex;
  flex-direction: column;
  gap: 14px;
  height: 100%;
  min-height: 0;
}
.topbar {
  flex-shrink: 0;
}
.info-card,
.log-card {
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
  color: #303133;
}
.info-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: #606266;
}
.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.lbl {
  width: 32px;
  color: #909399;
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
  font-family: 'Consolas', 'Courier New', monospace;
  color: #303133;
}
.metrics-wrap {
  margin-top: 10px;
  padding: 8px 10px;
  background: #f5f7fa;
  border-radius: 4px;
}

/* 日志卡片占满剩余高度 */
.log-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.log-card :deep(.el-card__body) {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  padding: 0;
}
.log-header {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
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
  color: #909399;
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
  font-family: 'Consolas', 'Courier New', monospace;
  font-size: 12.5px;
  line-height: 1.5;
  color: #303133;
  background: #fafafa;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
