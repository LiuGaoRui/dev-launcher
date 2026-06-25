<script setup lang="ts">
// 项目卡片底部的操作条。
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
  restart: []
  build: []
  deploy: []
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
    <el-button
      size="small"
      :disabled="busy || !canBuild"
      :title="canBuild ? '执行构建命令' : '未配置构建命令'"
      @click="emit('build')"
    >
      构建
    </el-button>
    <el-button
      size="small"
      type="primary"
      plain
      :disabled="busy || !canBuild"
      :title="canBuild ? '停止 → 构建 → 启动' : '未配置构建命令'"
      @click="emit('deploy')"
    >
      发布
    </el-button>
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
