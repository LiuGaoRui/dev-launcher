<script setup lang="ts">
// 根布局壳 —— 桌面软件三段式骨架。
//
//   ┌──────────────────────────────────────────────┐
//   │              TitleBar (36px)                 │
//   ├──────────┬───────────────────────────────────┤
//   │ Sidebar  │                                   │
//   │ (200px)  │          <slot/> 主内容区          │
//   │          │                                   │
//   ├──────────┴───────────────────────────────────┤
//   │              StatusBar (24px)                │
//   └──────────────────────────────────────────────┘

import TitleBar from './TitleBar.vue'
import AppSidebar from './AppSidebar.vue'
import StatusBar from './StatusBar.vue'
</script>

<template>
  <div class="app-shell">
    <TitleBar />
    <div class="app-body">
      <AppSidebar />
      <main class="app-main">
        <slot />
      </main>
    </div>
    <StatusBar />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--content-bg);
  overflow: hidden;
}
.app-body {
  flex: 1;
  display: flex;
  min-height: 0;
}
.app-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
}
</style>
