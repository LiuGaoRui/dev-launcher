<script setup lang="ts">
// 项目运行状态徽标。
//
// 阶段 3 简化态：仅 Running / Stopped 二态（runningIds 跟踪）。
// 阶段 5 接入端口探测后扩展为 Running / Running(Abnormal) / Stopped 等。
defineProps<{
  running: boolean
}>()
</script>

<template>
  <el-tag :type="running ? 'success' : 'info'" effect="light" size="small" round>
    <span class="dot" :class="{ on: running }" />
    {{ running ? '运行中' : '已停止' }}
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
