<script setup lang="ts">
// 应用根组件 —— 负责 Naive UI 全局 Provider 编排 + 布局壳挂载。
//
// Naive UI 要求 n-message-provider / n-dialog-provider 等必须是组件树祖先，
// 才能让后代组件通过 useMessage() / useDialog() 取到上下文。
// 因此 Provider 包在最外层（App.vue），布局壳 AppShell 放在其内部。

import { computed } from 'vue'
import {
  NConfigProvider,
  NMessageProvider,
  NDialogProvider,
  NLoadingBarProvider,
  zhCN,
  dateZhCN,
  type GlobalThemeOverrides,
} from 'naive-ui'
import { useThemeStore } from '@/stores/theme'
import AppShell from '@/components/layout/AppShell.vue'

const themeStore = useThemeStore()

/** 全局主题覆盖：统一品牌强调色、圆角、紧凑度，贴合桌面软件 */
const themeOverrides = computed<GlobalThemeOverrides>(() => ({
  common: {
    primaryColor: '#1890ff',
    primaryColorHover: '#40a9ff',
    primaryColorPressed: '#096dd9',
    primaryColorSuppl: '#1890ff',
    borderRadius: '5px',
    borderRadiusSmall: '4px',
    fontFamily:
      "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'PingFang SC', 'Microsoft YaHei', sans-serif",
    fontSize: '13px',
  },
  Button: {
    fontWeight: '400',
  },
}))
</script>

<template>
  <NConfigProvider
    :theme="themeStore.naiveTheme"
    :theme-overrides="themeOverrides"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <NLoadingBarProvider>
      <NDialogProvider>
        <NMessageProvider>
          <AppShell>
            <RouterView />
          </AppShell>
        </NMessageProvider>
      </NDialogProvider>
    </NLoadingBarProvider>
  </NConfigProvider>
</template>
