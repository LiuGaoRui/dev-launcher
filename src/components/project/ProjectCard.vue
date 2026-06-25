<script setup lang="ts">
// 单个项目卡片：展示名称/类型/路径/端口/PID，底部带操作条。
//
// 本组件只负责展示与抛事件，真正的 store 操作交给父组件（ProjectList），
// 这样卡片无状态、易测试、可复用。

import { PROJECT_TYPE_LABELS } from '@/types/project'
import type { Project } from '@/types/project'
import StatusBadge from './StatusBadge.vue'
import ActionBar from './ActionBar.vue'

const props = defineProps<{
  project: Project
  running: boolean
  /** 是否有进行中的启停操作（禁用按钮） */
  busy?: boolean
}>()

const emit = defineEmits<{
  start: []
  stop: []
  restart: []
  edit: []
  delete: []
  /** 点击卡片主体（进入详情，阶段 4+ 实现） */
  open: []
}>()

/** 端口列表展示为逗号分隔，空则显示 「-」 */
function portsText(p: Project): string {
  return p.expected_ports.length ? p.expected_ports.join(' / ') : '-'
}
</script>

<template>
  <el-card class="project-card" shadow="hover" :body-style="{ padding: '0' }">
    <div class="card-body" @click="emit('open')">
      <div class="card-head">
        <div class="title-row">
          <el-icon class="type-icon"><Monitor /></el-icon>
          <span class="name" :title="props.project.name">{{ props.project.name }}</span>
          <el-tag size="small" type="primary" effect="plain">
            {{ PROJECT_TYPE_LABELS[props.project.type] }}
          </el-tag>
          <StatusBadge :running="props.running" />
        </div>
      </div>

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
          <span v-if="props.project.last_pid" class="pid">PID {{ props.project.last_pid }}</span>
        </div>
      </div>

      <div class="actions" @click.stop>
        <ActionBar
          :running="props.running"
          :busy="props.busy"
          @start="emit('start')"
          @stop="emit('stop')"
          @restart="emit('restart')"
          @edit="emit('edit')"
          @delete="emit('delete')"
        />
      </div>
    </div>
  </el-card>
</template>

<style scoped>
.project-card {
  border-radius: 8px;
  transition: transform 0.15s ease;
}
.project-card:hover {
  transform: translateY(-2px);
}
.card-body {
  padding: 14px 16px;
  cursor: pointer;
}
.card-head {
  margin-bottom: 10px;
}
.title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.type-icon {
  font-size: 18px;
  color: #409eff;
}
.name {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: #606266;
  margin-bottom: 12px;
}
.meta-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.meta-label {
  width: 32px;
  color: #909399;
  flex-shrink: 0;
}
.meta-value {
  flex: 1;
  min-width: 0;
}
.meta-value.code {
  font-family: 'Consolas', 'Courier New', monospace;
  color: #303133;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pid {
  font-size: 11px;
  color: #67c23a;
  background: rgba(103, 194, 58, 0.1);
  padding: 1px 6px;
  border-radius: 3px;
}
.actions {
  border-top: 1px dashed #ebeef5;
  padding-top: 10px;
}
</style>
