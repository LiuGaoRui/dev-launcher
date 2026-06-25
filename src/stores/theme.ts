// 主题 Store —— 浅色 / 深色双主题切换。
//
// - 持久化到 localStorage，首次启动读取已保存偏好
// - 首次无偏好时跟随系统 prefers-color-scheme
// - 切换时同步 <html data-theme="...">，供非 Naive 组件（终端/标题栏/侧栏）感知
// - naiveTheme 派生值供 n-config-provider 绑定

import { computed, ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { darkTheme, type GlobalTheme } from 'naive-ui'

export type ThemeMode = 'light' | 'dark'

const STORAGE_KEY = 'devlauncher:theme'

function readStored(): ThemeMode | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY)
    if (v === 'light' || v === 'dark') return v
  } catch {
    /* localStorage 不可用时忽略 */
  }
  return null
}

function systemPrefersDark(): boolean {
  return (
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-color-scheme: dark)').matches
  )
}

function applyDom(mode: ThemeMode) {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', mode)
  }
}

export const useThemeStore = defineStore('theme', () => {
  // 初始化：已保存 > 系统 > 浅色
  const stored = readStored()
  const mode = ref<ThemeMode>(stored ?? (systemPrefersDark() ? 'dark' : 'light'))

  // 首次同步 DOM
  applyDom(mode.value)

  /** Naive UI theme 对象：浅色为 null，深色为 darkTheme */
  const naiveTheme = computed<GlobalTheme | null>(() =>
    mode.value === 'dark' ? darkTheme : null,
  )

  const isDark = computed(() => mode.value === 'dark')

  function setMode(next: ThemeMode) {
    mode.value = next
  }

  function toggle() {
    setMode(mode.value === 'dark' ? 'light' : 'dark')
  }

  // 持久化 + DOM 同步
  watch(mode, (m) => {
    try {
      localStorage.setItem(STORAGE_KEY, m)
    } catch {
      /* 忽略写入失败 */
    }
    applyDom(m)
  })

  return { mode, isDark, naiveTheme, setMode, toggle }
})
