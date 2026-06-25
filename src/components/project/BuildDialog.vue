<script setup lang="ts">
// 构建日志对话框 —— 阶段 7。
//
// 展示 build store 的实时输出（stdout 默认色 / stderr 红色），底部状态条：
// 进行中（转圈）/ 成功（绿）/ 失败（红）/ 命令错误。
// 关闭对话框 → reset 释放 Channel（GC → 后端读取 task 退出 → kill_on_drop 回收）。

import { computed, nextTick, ref, watch } from 'vue'
import { NModal, NIcon, NButton } from 'naive-ui'
import {
  CheckmarkCircleOutline,
  CloseCircleOutline,
  RefreshOutline,
} from '@vicons/ionicons5'
import { useBuildStore } from '@/stores/build'

const props = defineProps<{
  /** v-model 控制显隐 */
  modelValue: boolean
  /** 项目名称（标题展示用） */
  projectName: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

const buildStore = useBuildStore()

/** 输出区 DOM 引用（自动滚到底部） */
const outBox = ref<HTMLDivElement | null>(null)

const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v),
})

/** 状态：succeeded / failed / running / error */
const phase = computed<'running' | 'succeeded' | 'failed' | 'error'>(() => {
  if (buildStore.error) return 'error'
  if (buildStore.running) return 'running'
  return buildStore.exitCode === 0 ? 'succeeded' : 'failed'
})

/** 状态文案 */
const statusText = computed(() => {
  switch (phase.value) {
    case 'running':
      return '构建中…'
    case 'succeeded':
      return `构建成功（退出码 0，耗时 ${buildStore.durationMs ?? 0}ms）`
    case 'failed':
      return `构建失败（退出码 ${buildStore.exitCode}，耗时 ${buildStore.durationMs ?? 0}ms）`
    case 'error':
      return buildStore.error
  }
})

/** 新输出自动滚到底 */
watch(
  () => buildStore.lines.length,
  async () => {
    await nextTick()
    const box = outBox.value
    if (box) box.scrollTop = box.scrollHeight
  },
)

/** 对话框关闭：重置 store 释放订阅 */
function handleClose() {
  buildStore.reset()
}
</script>

<template>
  <NModal
    v-model:show="visible"
    preset="card"
    :title="`构建「${projectName}」`"
    style="width: 80vw; max-width: 1100px"
    :mask-closable="false"
    @after-leave="handleClose"
  >
    <div class="build-output" ref="outBox">
      <span
        v-for="(line, i) in buildStore.lines"
        :key="i"
        class="line"
        :class="{ err: line.kind === 'stderr' }"
      >{{ line.text }}</span>
      <span v-if="!buildStore.lines.length && !buildStore.error" class="placeholder">
        （等待输出…）
      </span>
    </div>

    <template #footer>
      <div class="footer">
        <div class="status" :class="phase">
          <NIcon v-if="phase === 'running'" class="spin"><RefreshOutline /></NIcon>
          <NIcon v-else-if="phase === 'succeeded'"><CheckmarkCircleOutline /></NIcon>
          <NIcon v-else><CloseCircleOutline /></NIcon>
          <span>{{ statusText }}</span>
        </div>
        <NButton :disabled="buildStore.running" @click="visible = false">关闭</NButton>
      </div>
    </template>
  </NModal>
</template>

<style scoped>
.build-output {
  height: 50vh;
  min-height: 240px;
  overflow: auto;
  background: var(--terminal-bg);
  border-radius: 6px;
  padding: 12px 14px;
  font-family: var(--code-font);
  font-size: 12.5px;
  line-height: 1.55;
  color: var(--terminal-fg);
  white-space: pre-wrap;
  word-break: break-all;
}
.line {
  white-space: pre-wrap;
}
.line.err {
  color: var(--terminal-err);
}
.placeholder {
  color: var(--terminal-dim);
}
.footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
}
.status.running {
  color: var(--accent);
}
.status.succeeded {
  color: var(--status-running);
}
.status.failed,
.status.error {
  color: var(--terminal-err);
}
.spin {
  animation: rotate 1.2s linear infinite;
}
@keyframes rotate {
  to {
    transform: rotate(360deg);
  }
}
</style>
