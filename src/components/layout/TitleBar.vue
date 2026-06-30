<script setup lang="ts">
// 集成无边框标题栏。
//
// 三段布局：
//   左：应用图标 + 名称（可点击区域，但不可拖动整窗避免误触）
//   中：拖动区域（data-tauri-drag-region）—— 双击切换最大化
//   右：清理入口 + 主题切换开关 + Windows 三按钮（最小化 / 最大化-还原 / 关闭）
//
// 依赖 tauri.conf.json decorations:false，窗口本身已无系统标题栏。
// Tauri 的 drag region 通过 data-tauri-drag-region 属性自动接管拖动。

import { onMounted, onBeforeUnmount, ref } from 'vue'
import { NIcon } from 'naive-ui'
import {
  RemoveOutline,
  ExpandOutline,
  ContractOutline,
  CloseOutline,
  MoonOutline,
  SunnyOutline,
  ServerOutline,
} from '@vicons/ionicons5'
import { useThemeStore } from '@/stores/theme'
import {
  minimizeWindow,
  toggleMaximize,
  closeWindow,
  onMaximizedChange,
} from '@/utils/window'
import CleanerDrawer from '@/components/cleaner/CleanerDrawer.vue'

const themeStore = useThemeStore()

/** 当前是否最大化（驱动按钮图标：Expand ↔ Contract） */
const maximized = ref(false)

/** 内存清理抽屉开关 */
const cleanerVisible = ref(false)

let cleanup: (() => void) | null = null

onMounted(async () => {
  cleanup = await onMaximizedChange((m) => {
    maximized.value = m
  })
})

onBeforeUnmount(() => {
  cleanup?.()
})
</script>

<template>
  <div class="titlebar">
    <!-- 左：应用标识 -->
    <div class="brand">
      <span class="logo-dot" />
      <span class="brand-name">DevLauncher</span>
    </div>

    <!-- 中：拖动区域（双击切换最大化由系统处理） -->
    <div class="drag-region" data-tauri-drag-region />

    <!-- 右：清理入口 + 主题切换 + 窗口控制 -->
    <div class="trailing">
      <button class="tool-btn" title="内存清理" @click="cleanerVisible = true">
        <NIcon size="15"><ServerOutline /></NIcon>
      </button>
      <NIcon
        v-if="themeStore.isDark"
        class="theme-icon"
        size="14"
        @click="themeStore.setMode('light')"
      >
        <SunnyOutline />
      </NIcon>
      <NIcon
        v-else
        class="theme-icon"
        size="14"
        @click="themeStore.setMode('dark')"
      >
        <MoonOutline />
      </NIcon>

      <button class="win-btn" title="最小化" @click="minimizeWindow">
        <NIcon size="14"><RemoveOutline /></NIcon>
      </button>
      <button class="win-btn" :title="maximized ? '还原' : '最大化'" @click="toggleMaximize">
        <NIcon size="13">
          <ContractOutline v-if="maximized" />
          <ExpandOutline v-else />
        </NIcon>
      </button>
      <button class="win-btn close" title="关闭" @click="closeWindow">
        <NIcon size="14"><CloseOutline /></NIcon>
      </button>
    </div>

    <!-- 内存清理抽屉 -->
    <CleanerDrawer v-model:show="cleanerVisible" />
  </div>
</template>

<style scoped>
.titlebar {
  height: 36px;
  display: flex;
  align-items: center;
  background: var(--titlebar-bg);
  color: var(--titlebar-fg);
  border-bottom: 1px solid var(--divider);
  user-select: none;
  flex-shrink: 0;
}

/* 左侧品牌区（不作为 drag region，避免点击误触拖动） */
.brand {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 14px;
  flex-shrink: 0;
  height: 100%;
}
.logo-dot {
  width: 14px;
  height: 14px;
  border-radius: 4px;
  background: linear-gradient(135deg, #1890ff, #52c41a);
  flex-shrink: 0;
}
.brand-name {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.3px;
}

/* 中间拖动区域占满剩余空间 */
.drag-region {
  flex: 1;
  height: 100%;
}

/* 右侧功能区 */
.trailing {
  display: flex;
  align-items: center;
  height: 100%;
  flex-shrink: 0;
}
.theme-icon {
  cursor: pointer;
  /* 与 .win-btn 一致的盒模型，使图标与窗口三按钮等大 */
  width: 46px;
  height: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  /* 与窗口三按钮拉开间距 */
  margin-right: 8px;
  color: var(--titlebar-fg);
  opacity: 0.75;
  transition: opacity 0.15s;
}
.theme-icon:hover {
  opacity: 1;
}

/* 功能按钮（清理等工具入口），比主题图标窄一些 */
.tool-btn {
  width: 36px;
  height: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--titlebar-fg);
  cursor: pointer;
  opacity: 0.7;
  transition: opacity 0.15s, background 0.12s;
  border-radius: 4px;
}
.tool-btn:hover {
  opacity: 1;
  background: var(--titlebar-btn-hover);
}

/* Windows 风窗口控制按钮 */
.win-btn {
  height: 100%;
  width: 46px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--titlebar-fg);
  cursor: pointer;
  transition: background 0.12s;
}
.win-btn:hover {
  background: var(--titlebar-btn-hover);
}
.win-btn.close:hover {
  background: var(--titlebar-close-hover);
  color: var(--titlebar-close-fg);
}
</style>
