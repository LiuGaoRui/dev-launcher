<script setup lang="ts">
// 项目健康状态徽标（阶段 5 三态）。
//
// - running：绿色脉动点
// - running_abnormal：橙色 + 警告图标，tooltip 说明端口未就绪/被占用
// - stopped：灰色
import { computed } from 'vue'
import { Warning } from '@element-plus/icons-vue'
import { HEALTH_LABELS, type HealthStatus } from '@/types/monitor'

const TAG_TYPE_MAP: Record<HealthStatus, 'success' | 'warning' | 'info'> = {
  running: 'success',
  running_abnormal: 'warning',
  stopped: 'info',
}

const props = defineProps<{
  health: HealthStatus
}>()

const tagType = computed(() => TAG_TYPE_MAP[props.health])
const isAbnormal = computed(() => props.health === 'running_abnormal')
const isRunning = computed(() => tagType.value !== 'info')
</script>

<template>
  <el-tag :type="tagType" effect="light" size="small" round>
    <span class="dot" :class="{ on: isRunning, warn: isAbnormal }" />
    <el-icon v-if="isAbnormal" class="warn-icon"><Warning /></el-icon>
    {{ HEALTH_LABELS[props.health] }}
  </el-tag>
</template>

<style scoped>
.dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  margin-right: 4px;
  vertical-align: middle;
  background: #909399;
}
.dot.on {
  background: #67c23a;
  box-shadow: 0 0 0 3px rgba(103, 194, 58, 0.2);
  animation: pulse 1.6s ease-in-out infinite;
}
.dot.warn {
  background: #e6a23c;
  box-shadow: 0 0 0 3px rgba(230, 162, 60, 0.2);
}
.warn-icon {
  margin-right: 2px;
  font-size: 12px;
  vertical-align: middle;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.55;
  }
}
</style>
