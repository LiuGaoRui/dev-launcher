<script setup lang="ts">
// 桌面风侧边栏导航。
//
// - 顶部：紧凑应用标识（标题栏已含完整品牌名，此处用短标识）
// - 中部：图标 + 文字导航（n-menu），跟随路由高亮
// - 底部：运行态统计卡（运行中 / 总数），实时来自 projectStore

import { computed, h, type Component } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NIcon, NMenu, type MenuOption } from 'naive-ui'
import { CubeOutline, FolderOpenOutline } from '@vicons/ionicons5'
import { useProjectStore } from '@/stores/project'

const route = useRoute()
const router = useRouter()
const projectStore = useProjectStore()

/** 渲染 vicons 图标为 n-menu 的 icon */
function renderIcon(icon: Component) {
  return () => h(NIcon, null, { default: () => h(icon) })
}

const activeKey = computed(() => route.path)

const menuOptions = computed<MenuOption[]>(() => [
  {
    label: '项目列表',
    key: '/projects',
    icon: renderIcon(CubeOutline),
  },
  {
    label: '分组管理',
    key: '/groups',
    icon: renderIcon(FolderOpenOutline),
  },
])

function handleSelect(key: string) {
  router.push(key)
}
</script>

<template>
  <aside class="sidebar">
    <!-- 导航区 -->
    <div class="nav-wrap">
      <NMenu
        :value="activeKey"
        :options="menuOptions"
        :indent="18"
        :collapsed-width="200"
        :collapsed-icon-size="18"
        @update:value="handleSelect"
      />
    </div>

    <!-- 底部统计卡 -->
    <div class="stats">
      <div class="stat-card">
        <div class="stat-line">
          <span class="status-dot" :class="{ 'status-dot--on': projectStore.runningCount > 0 }" />
          <span class="stat-label">运行中</span>
          <span class="stat-value run">{{ projectStore.runningCount }}</span>
        </div>
        <div class="stat-line">
          <span class="status-dot" />
          <span class="stat-label">项目总数</span>
          <span class="stat-value">{{ projectStore.totalCount }}</span>
        </div>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 200px;
  background: var(--sidebar-bg);
  border-right: 1px solid var(--divider);
  display: flex;
  flex-direction: column;
  height: 100%;
  user-select: none;
  flex-shrink: 0;
}

.nav-wrap {
  flex: 1;
  padding-top: 8px;
  overflow-y: auto;
}

/* 底部统计卡 */
.stats {
  padding: 10px 12px 14px;
  border-top: 1px solid var(--divider);
}
.stat-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.stat-line {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-secondary);
}
.stat-label {
  flex: 1;
}
.stat-value {
  font-family: var(--code-font);
  font-weight: 600;
  color: var(--text-primary);
}
.stat-value.run {
  color: var(--status-running);
}
</style>
