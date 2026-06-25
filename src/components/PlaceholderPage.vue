<script setup lang="ts">
import { NEmpty } from 'naive-ui'

defineProps<{
  description: string
}>()
</script>

<template>
  <div class="placeholder">
    <NEmpty :description="description" />
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
