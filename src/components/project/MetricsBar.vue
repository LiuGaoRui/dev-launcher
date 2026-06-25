<script setup lang="ts">
// 运行中项目的指标条：CPU / 内存 / 端口状态。
//
// 仅在项目运行中（running / running_abnormal）时由父组件渲染。
// 端口徽标三态：listening&&owned 绿、listening&&!owned 橙(被占用)、!listening 灰。
import { formatBytes, type ProjectStatus } from '@/types/monitor'

const props = defineProps<{
  status: ProjectStatus
}>()
</script>

<template>
  <div class="metrics-bar">
    <span class="metric">
      <span class="m-label">CPU</span>
      <span class="m-value">{{ props.status.cpu_percent.toFixed(1) }}%</span>
    </span>
    <span class="metric">
      <span class="m-label">内存</span>
      <span class="m-value">{{ formatBytes(props.status.memory_bytes) }}</span>
    </span>
    <span v-if="props.status.ports.length" class="ports">
      <span
        v-for="p in props.status.ports"
        :key="p.port"
        class="port-item"
      >
        <span class="port-num">{{ p.port }}</span>
        <el-tag v-if="!p.listening" type="info" size="small" effect="plain">未监听</el-tag>
        <el-tag v-else-if="!p.owned" type="warning" size="small" effect="plain">被占用</el-tag>
        <el-tag v-else type="success" size="small" effect="plain">监听中</el-tag>
      </span>
    </span>
  </div>
</template>

<style scoped>
.metrics-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
  font-size: 12px;
  color: #606266;
}
.metric {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.m-label {
  color: #909399;
}
.m-value {
  font-family: 'Consolas', 'Courier New', monospace;
  color: #303133;
  font-weight: 500;
}
.ports {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.port-item {
  display: inline-flex;
  align-items: center;
  gap: 3px;
}
.port-num {
  font-family: 'Consolas', 'Courier New', monospace;
  color: #409eff;
  font-weight: 600;
}
</style>
