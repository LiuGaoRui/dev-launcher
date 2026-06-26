<script setup lang="ts">
// 底部状态栏（桌面软件标配）。
//
// 左：运行态汇总（● 3/12 运行中）
// 中：轮询状态指示
// 右：工具自身内存占用 + 版本号
//
// 运行态数据来自 projectStore（轮询状态实时刷新），无需自身拉取。
// 内存占用每 3s 轮询后端 get_app_memory（与项目监控周期一致）。

import { onMounted, onBeforeUnmount, ref } from 'vue'
import { useProjectStore, POLL_INTERVAL_MS } from '@/stores/project'
import { getAppMemory } from '@/api/system'
import { formatBytes } from '@/types/monitor'

const projectStore = useProjectStore()

const APP_VERSION = 'v0.1.0'

/** 工具自身内存占用（字节） */
const appMemory = ref(0)

let memTimer: ReturnType<typeof setInterval> | undefined

async function refreshMemory() {
  try {
    appMemory.value = await getAppMemory()
  } catch (e) {
    // 内存读取失败不应打断状态栏；下次轮询重试
    console.warn('get_app_memory 失败:', e)
  }
}

onMounted(() => {
  void refreshMemory()
  memTimer = setInterval(() => void refreshMemory(), POLL_INTERVAL_MS)
})

onBeforeUnmount(() => {
  clearInterval(memTimer)
  memTimer = undefined
})
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
    <div class="seg right">
      <span class="mem">内存 {{ formatBytes(appMemory) }}</span>
      <span>DevLauncher {{ APP_VERSION }}</span>
    </div>
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
  gap: 14px;
}
.seg.right .mem {
  opacity: 0.75;
}

.poll-indicator {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  animation: pulse 2s ease-in-out infinite;
}
</style>
