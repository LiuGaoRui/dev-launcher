<script setup lang="ts">
// 项目卡片 —— 桌面风紧凑卡片。
//
// 视觉：
//   左侧 3px 状态色条（绿/橙/灰，随健康状态变色）
//   hover 时边框高亮 + 背景微变（移除网页式 translateY 上浮）
//   紧凑密度，圆角 6px
//
// 交互：点击卡片主体进入详情；底部操作条 stop 冒泡。
// 本组件只负责展示与抛事件，store 操作交给父组件（ProjectList），卡片无状态。
import { computed } from 'vue'
import { NIcon, NTag } from 'naive-ui'
import { CubeOutline } from '@vicons/ionicons5'
import { PROJECT_TYPE_LABELS } from '@/types/project'
import type { Project } from '@/types/project'
import type { HealthStatus, ProjectStatus } from '@/types/monitor'
import StatusBadge from './StatusBadge.vue'
import MetricsBar from './MetricsBar.vue'
import ActionBar from './ActionBar.vue'

const props = defineProps<{
  project: Project
  /** 运行态探测结果（null = 未运行/stopped） */
  status?: ProjectStatus | null
  /** 是否有进行中的启停操作（禁用按钮） */
  busy?: boolean
}>()

const emit = defineEmits<{
  start: []
  stop: []
  restart: []
  build: []
  deploy: []
  edit: []
  delete: []
  /** 点击卡片主体（进入详情） */
  open: []
}>()

/** 健康状态：有探测结果用其 health，否则 stopped */
const health = computed<HealthStatus>(() => props.status?.health ?? 'stopped')

/** 是否运行中（running 或 running_abnormal） */
const running = computed(() => health.value !== 'stopped')

/** 实时 PID（优先 status.pid，回退 project.last_pid 缓存） */
const pid = computed(() => props.status?.pid ?? props.project.last_pid ?? null)

/** 是否配置了构建命令（控制构建/发布按钮可用性） */
const canBuild = computed(() => !!props.project.build_cmd?.trim())

/** 状态色条 class */
const stateClass = computed(() => `state-${health.value}`)

/** 端口列表展示为斜杠分隔，空则显示「-」 */
function portsText(p: Project): string {
  return p.expected_ports.length ? p.expected_ports.join(' / ') : '-'
}
</script>

<template>
  <div class="project-card" :class="stateClass" @click="emit('open')">
    <!-- 头部：图标 + 名称 + 类型标签 + 状态 -->
    <div class="card-head">
      <NIcon class="type-icon" size="16">
        <CubeOutline />
      </NIcon>
      <span class="name" :title="props.project.name">{{ props.project.name }}</span>
      <NTag size="tiny" type="primary" :bordered="false">
        {{ PROJECT_TYPE_LABELS[props.project.type] }}
      </NTag>
      <StatusBadge :health="health" class="status-badge" />
    </div>

    <!-- 元数据 -->
    <div class="meta">
      <div class="meta-row" :title="props.project.path">
        <span class="meta-label">路径</span>
        <span class="meta-value ellipsis">{{ props.project.path || '-' }}</span>
      </div>
      <div class="meta-row">
        <span class="meta-label">启动</span>
        <span class="meta-value code" :title="props.project.start_cmd">
          {{ props.project.start_cmd || '-' }}
        </span>
      </div>
      <div class="meta-row">
        <span class="meta-label">端口</span>
        <span class="meta-value">{{ portsText(props.project) }}</span>
        <span v-if="pid" class="pid">PID {{ pid }}</span>
      </div>
    </div>

    <!-- 运行中实时指标 -->
    <div v-if="props.status && running" class="metrics-wrap">
      <MetricsBar :status="props.status" />
    </div>

    <!-- 操作条 -->
    <div class="actions" @click.stop>
      <ActionBar
        :running="running"
        :busy="props.busy"
        :can-build="canBuild"
        @start="emit('start')"
        @stop="emit('stop')"
        @restart="emit('restart')"
        @build="emit('build')"
        @deploy="emit('deploy')"
        @edit="emit('edit')"
        @delete="emit('delete')"
      />
    </div>
  </div>
</template>

<style scoped>
.project-card {
  position: relative;
  background: var(--card-bg);
  border: 1px solid var(--card-border);
  border-radius: 6px;
  padding: 12px 14px 12px 16px;
  cursor: pointer;
  transition: border-color 0.12s, background 0.12s;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 左侧状态色条 */
.project-card::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 0 2px 2px 0;
  background: var(--status-stopped);
}
.project-card.state-running::before {
  background: var(--status-running);
}
.project-card.state-running_abnormal::before {
  background: var(--status-warning);
}

/* hover：边框高亮 + 背景微变（无上浮动效） */
.project-card:hover {
  border-color: var(--card-hover-border);
  background: var(--card-hover-bg);
}

/* 头部 */
.card-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.type-icon {
  color: var(--accent);
  flex-shrink: 0;
}
.name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 0;
}
.status-badge {
  margin-left: auto;
  flex-shrink: 0;
}

/* 元数据 */
.meta {
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
.meta-label {
  width: 30px;
  color: var(--text-tertiary);
  flex-shrink: 0;
  font-size: 11.5px;
}
.meta-value {
  flex: 1;
  min-width: 0;
}
.meta-value.code {
  font-family: var(--code-font);
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 运行指标 */
.metrics-wrap {
  padding: 7px 9px;
  background: var(--status-running-soft);
  border-radius: 4px;
}

/* PID 徽标 */
.pid {
  font-size: 11px;
  color: var(--status-running);
  background: var(--status-running-soft);
  padding: 1px 6px;
  border-radius: 3px;
  font-family: var(--code-font);
  flex-shrink: 0;
}

/* 操作条 */
.actions {
  border-top: 1px dashed var(--divider);
  padding-top: 10px;
  margin: 0 -2px;
}
</style>
