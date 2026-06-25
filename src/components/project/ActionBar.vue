<script setup lang="ts">
// 项目卡片底部的操作条。
// 阶段 3：启动 / 停止 / 重启 / 编辑 / 删除（构建/发布留待阶段 7）。

defineProps<{
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
}>()
</script>

<template>
  <div class="action-bar">
    <el-button
      v-if="!running"
      type="success"
      size="small"
      :loading="busy"
      @click="emit('start')"
    >
      启动
    </el-button>
    <el-button
      v-else
      type="warning"
      size="small"
      :loading="busy"
      @click="emit('stop')"
    >
      停止
    </el-button>
    <el-button size="small" :disabled="busy" @click="emit('restart')">重启</el-button>
    <el-button size="small" :disabled="busy" text @click="emit('edit')">编辑</el-button>
    <el-button
      size="small"
      type="danger"
      text
      :disabled="busy || running"
      :title="running ? '请先停止项目再删除' : '删除项目'"
      @click="emit('delete')"
    >
      删除
    </el-button>
  </div>
</template>

<style scoped>
.action-bar {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
</style>
