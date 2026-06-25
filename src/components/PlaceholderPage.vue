<script setup lang="ts">
defineProps<{
  description: string
}>()
</script>

<template>
  <div class="placeholder">
    <el-empty :description="description" />
  </div>
</template>

<style scoped>
.placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
}
</style>
