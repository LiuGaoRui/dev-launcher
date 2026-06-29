<script setup lang="ts">
// 项目卡片底部的操作条。
import { NButton } from 'naive-ui'
// 阶段 3：启动 / 停止 / 重启 / 编辑 / 删除。
// 阶段 7：新增 构建 / 发布（build_cmd 为空时禁用，前端无法感知 build_cmd，
//          由父组件用 canBuild prop 控制可用性）。

defineProps<{
  running: boolean
  /** 是否有进行中的启停操作（禁用按钮） */
  busy?: boolean
  /** 是否配置了构建命令（控制构建/发布按钮可用性） */
  canBuild?: boolean
}>()

const emit = defineEmits<{
  start: []
  stop: []
  build: []
  edit: []
  delete: []
}>()
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
      :disabled="busy || !canBuild"
      :title="canBuild ? '执行构建命令' : '未配置构建命令'"
      @click="emit('build')"
    >
      构建
    </NButton>
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
</style>
