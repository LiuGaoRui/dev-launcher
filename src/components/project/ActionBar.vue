<script setup lang="ts">
// 项目卡片底部的操作条。
// 按钮：启动/停止 | 构建(+状态图标) | 日志 | 编辑 | 删除
//
// 构建状态图标（由父组件用 buildState prop 传入）：
//   running → 旋转图标；失败(exit_code!=0 且非 running) → 红色 X；否则无图标。

import { computed } from 'vue'
import { NButton, NIcon } from 'naive-ui'
import { RefreshOutline, CloseCircle } from '@vicons/ionicons5'
import type { BuildState } from '@/types/project'

const props = defineProps<{
  running: boolean
  /** 是否有进行中的启停操作（禁用按钮） */
  busy?: boolean
  /** 是否配置了构建命令（控制构建按钮可用性） */
  canBuild?: boolean
  /** 构建状态（驱动构建按钮后的状态图标） */
  buildState?: BuildState
}>()

const emit = defineEmits<{
  start: []
  stop: []
  build: []
  log: []
  edit: []
  delete: []
}>()

/** 构建是否进行中 */
const buildRunning = computed(() => props.buildState?.running ?? false)
/** 构建是否失败（非进行中 + 退出码非 0 + 有退出码记录） */
const buildFailed = computed(
  () =>
    !buildRunning.value &&
    !!props.buildState &&
    (props.buildState.exit_code !== 0 || !!props.buildState.error),
)
</script>

<template>
  <div class="action-bar">
    <NButton
      v-if="!running"
      size="tiny"
      type="success"
      :loading="busy"
      @click="emit('start')"
    >
      启动
    </NButton>
    <NButton
      v-else
      size="tiny"
      type="warning"
      :loading="busy"
      @click="emit('stop')"
    >
      停止
    </NButton>
    <NButton
      size="tiny"
      tertiary
      :disabled="busy || !canBuild || buildRunning"
      :title="canBuild ? '执行构建命令' : '未配置构建命令'"
      @click="emit('build')"
    >
      <template #icon>
        <!-- 构建中：旋转图标 -->
        <NIcon v-if="buildRunning" class="spin">
          <RefreshOutline />
        </NIcon>
        <!-- 构建失败：红色 X -->
        <NIcon v-else-if="buildFailed" class="fail">
          <CloseCircle />
        </NIcon>
      </template>
      构建
    </NButton>
    <NButton size="tiny" tertiary @click="emit('log')">日志</NButton>
    <NButton size="tiny" quaternary :disabled="busy" @click="emit('edit')">编辑</NButton>
    <NButton size="tiny" quaternary type="error" :disabled="busy" @click="emit('delete')">
      删除
    </NButton>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  gap: 6px;
  align-items: center;
  flex-wrap: wrap;
}
/* 构建中旋转动画 */
.spin {
  animation: rotate 1.2s linear infinite;
}
@keyframes rotate {
  to {
    transform: rotate(360deg);
  }
}
/* 构建失败图标红色 */
.fail {
  color: var(--status-error, #d03050);
}
</style>
