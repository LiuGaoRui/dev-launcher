<script setup lang="ts">
import { Monitor, Folder } from '@element-plus/icons-vue'
</script>

<template>
  <el-container class="app-container">
    <el-aside width="200px" class="app-aside">
      <div class="logo">
        <span>DevLauncher</span>
      </div>
      <el-menu
        :default-active="$route.path"
        router
        class="app-menu"
      >
        <el-menu-item index="/projects">
          <el-icon><Monitor /></el-icon>
          <span>项目列表</span>
        </el-menu-item>
        <el-menu-item index="/groups">
          <el-icon><Folder /></el-icon>
          <span>分组管理</span>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="app-header">
        <span class="header-title">DevLauncher — 开发机项目管理器</span>
      </el-header>
      <el-main class="app-main">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<style scoped>
.app-container {
  height: 100vh;
}

.app-aside {
  background: #001529;
  display: flex;
  flex-direction: column;
}

.logo {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 20px;
  font-weight: bold;
  border-bottom: 1px solid #1f1f1f;
}

.app-menu {
  flex: 1;
  border-right: none;
  background: transparent;
}

.app-header {
  display: flex;
  align-items: center;
  border-bottom: 1px solid #e8e8e8;
  background: #fff;
}

.header-title {
  font-size: 16px;
  font-weight: 600;
  color: #333;
}

.app-main {
  background: #f5f5f5;
  padding: 20px;
}
</style>
