<script setup lang="ts">
// 项目卡片底部的操作条。
// 按钮：启动/停止 | 重启 | 构建(+状态图标)/停止构建 | 日志 | 编辑 | 删除
//
// 「重启」= 停止 → 构建（可选）→ 启动 三步串行（见 ProjectList.handleRestart）。
// 有 build_cmd 走完整三步；无 build_cmd（纯 dev 模式）走停止 → 启动。
//
// 构建状态图标（由父组件用 buildState prop 传入）：
//   running → 按钮变为「停止构建」（杀进程树）；失败(exit_code!=0 且非 running) → 红色 X；否则无图标。

import { computed } from 'vue'
import { NButton, NIcon } from 'naive-ui'
import { CloseCircle, StopOutline } from '@vicons/ionicons5'
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
  restart: []
  build: []
  stopBuild: []
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
      :disabled="busy || buildRunning"
      :title="canBuild ? '停止 → 构建 → 启动' : '停止 → 启动'"
      @click="emit('restart')"
    >
      重启
    </NButton>
    <!-- 构建中：变为「停止构建」按钮（兜底构建命令进程树不退出的场景）。
         刻意不受 busy 禁用：重启流程卡在构建阶段时它是唯一的救援出口 -->
    <NButton
      v-if="buildRunning"
      size="tiny"
      type="warning"
      tertiary
      title="强制终止构建进程树"
      @click="emit('stopBuild')"
    >
      <template #icon>
        <NIcon>
          <StopOutline />
        </NIcon>
      </template>
      停止构建
    </NButton>
    <NButton
      v-else
      size="tiny"
      tertiary
      :disabled="busy || !canBuild || buildRunning"
      :title="canBuild ? '执行构建命令' : '未配置构建命令'"
      @click="emit('build')"
    >
      <template #icon>
        <!-- 构建失败：红色 X -->
        <NIcon v-if="buildFailed" class="fail">
          <CloseCircle />
        </NIcon>
      </template>
      构建
    </NButton>
    <NButton size="tiny" tertiary :disabled="busy" @click="emit('log')">日志</NButton>
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
/* 构建失败图标红色 */
.fail {
  color: var(--status-error, #d03050);
}
</style>
