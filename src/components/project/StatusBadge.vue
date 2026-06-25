<script setup lang="ts">
// 项目健康状态徽标（阶段 5 三态）。
//
// - running：绿色脉动点
// - running_abnormal：橙色 + 警告图标，tooltip 说明端口未就绪/被占用
// - stopped：灰色
import { computed } from 'vue'
import { NTag, NIcon } from 'naive-ui'
import { WarningOutline } from '@vicons/ionicons5'
import { HEALTH_LABELS, type HealthStatus } from '@/types/monitor'

const TAG_TYPE_MAP: Record<HealthStatus, 'success' | 'warning' | 'default'> = {
  running: 'success',
  running_abnormal: 'warning',
  stopped: 'default',
}

const props = defineProps<{
  health: HealthStatus
}>()

const tagType = computed(() => TAG_TYPE_MAP[props.health])
const isAbnormal = computed(() => props.health === 'running_abnormal')
const isRunning = computed(() => tagType.value !== 'default')
</script>

<template>
  <NTag :type="tagType" size="small" round :bordered="false">
    <template #icon>
      <NIcon v-if="isAbnormal"><WarningOutline /></NIcon>
    </template>
    <span class="dot" :class="{ on: isRunning, warn: isAbnormal }" />
    {{ HEALTH_LABELS[props.health] }}
  </NTag>
</template>

<style scoped>
.dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  margin-right: 4px;
  vertical-align: middle;
  background: var(--status-stopped);
}
.dot.on {
  background: var(--status-running);
  box-shadow: 0 0 0 3px var(--status-running-soft);
  animation: pulse 1.6s ease-in-out infinite;
}
.dot.warn {
  background: var(--status-warning);
  box-shadow: 0 0 0 3px var(--status-warning-soft);
}
</style>
