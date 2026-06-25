<script setup lang="ts">
// 底部状态栏（桌面软件标配）。
//
// 左：运行态汇总（● 3/12 运行中）
// 中：轮询状态指示
// 右：版本号
//
// 数据来自 projectStore（轮询状态实时刷新），无需自身拉取。

import { useProjectStore } from '@/stores/project'

const projectStore = useProjectStore()

const APP_VERSION = 'v0.1.0'
</script>

<template>
  <footer class="statusbar">
    <div class="seg">
      <span class="status-dot" :class="{ 'status-dot--on': projectStore.runningCount > 0 }" />
      <span>{{ projectStore.runningCount }}/{{ projectStore.totalCount }} 运行中</span>
    </div>
    <div class="seg center">
      <span class="poll-indicator" /> 实时监控
    </div>
    <div class="seg right">DevLauncher {{ APP_VERSION }}</div>
  </footer>
</template>

<style scoped>
.statusbar {
  height: 24px;
  display: flex;
  align-items: center;
  background: var(--statusbar-bg);
  color: var(--statusbar-fg);
  border-top: 1px solid var(--divider);
  font-size: 11.5px;
  padding: 0 12px;
  user-select: none;
  flex-shrink: 0;
  gap: 16px;
}

.seg {
  display: flex;
  align-items: center;
  gap: 6px;
}
.seg.center {
  flex: 1;
  justify-content: center;
  opacity: 0.7;
}
.seg.right {
  font-family: var(--code-font);
}

.poll-indicator {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  animation: pulse 2s ease-in-out infinite;
}
</style>
