<script setup lang="ts">
// 端口监控弹窗 —— 全系统 TCP LISTEN 端口 + 分类评分展示 + 一键结束进程。
//
// 布局（NModal preset card，880px，内容区最高 calc(80vh - 72px)，卡片总高 ≈ 窗口 80%）：
//   顶部固定：分类统计条（可疑/开发/已配置/服务/系统 各档计数）+ 可疑提示
//   工具栏：搜索框 + 刷新
//   滚动区：重点区（可疑 + 开发 + 其他，后端 score 降序平铺，可疑行红色高亮）
//           + 折叠区（已配置项目端口 / 环境服务 / 系统端口 / 已忽略）
//
// 排序与分类全部由后端完成（port_scan.rs），前端只渲染分组视图。
// 「结束进程」复用 cleaner 的 kill_dev_processes（taskkill /F /T 杀整树，
// 含锁定表/自身护栏），行内二次确认后执行。
// 系统端口 / 环境服务 / 已配置-托管项目不提供杀按钮（防误杀，托管项目走项目页停止）。
// 可疑行可「忽略」（按端口号持久化到后端 port_ignore 表），忽略后移入
// 「已忽略」折叠区、不计入角标；「恢复」按钮可撤销。

import { computed, onBeforeUnmount, ref, watch } from 'vue'
import {
  NModal,
  NButton,
  NIcon,
  NInput,
  NTag,
  NEmpty,
  NSpin,
  useMessage,
  useDialog,
} from 'naive-ui'
import {
  SearchOutline,
  RefreshOutline,
  TrashOutline,
  ChevronDownOutline,
  ChevronForwardOutline,
  WarningOutline,
  EyeOffOutline,
} from '@vicons/ionicons5'
import { usePortStore } from '@/stores/port'
import {
  PORT_CATEGORY_META,
  formatRelativeTime,
  type ListeningPortInfo,
  type PortCategory,
} from '@/types/port'
import { formatBytes } from '@/types/monitor'
import { usePolling } from '@/composables/usePolling'

/** 弹窗打开期间的高频轮询间隔（ms） */
const POLL_MS = 5000

const props = defineProps<{ show: boolean }>()
const emit = defineEmits<{ 'update:show': [value: boolean] }>()

const portStore = usePortStore()
const message = useMessage()
const dialog = useDialog()

/** 搜索关键字（过滤端口/进程名/展示名/路径/摘要/项目名） */
const keyword = ref('')

/** 各折叠区展开状态（重点区常开） */
const collapsed = ref({
  knownProject: true, // 默认折叠
  envService: true,
  system: true,
  ignored: true,
})

/** 弹窗打开期间 5s 高频轮询（TitleBar 的 30s 低频轮询照常驱动角标） */
const { start: startPolling, stop: stopPolling } = usePolling(portStore.scan, POLL_MS)

watch(
  () => props.show,
  (open) => {
    if (open) startPolling()
    else stopPolling()
  },
)

onBeforeUnmount(stopPolling)

/** 关键字过滤（覆盖所有展示字段，空关键字返回原列表） */
function filterByKeyword(list: ListeningPortInfo[]): ListeningPortInfo[] {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return list
  return list.filter((p) => {
    return (
      String(p.port) === kw ||
      p.process_name.toLowerCase().includes(kw) ||
      p.display_title.toLowerCase().includes(kw) ||
      p.project_path.toLowerCase().includes(kw) ||
      p.cmdline_summary.toLowerCase().includes(kw) ||
      (p.matched_project_name ?? '').toLowerCase().includes(kw) ||
      p.addresses.some((a) => a.toLowerCase().includes(kw))
    )
  })
}

const focusList = computed(() => filterByKeyword(portStore.focusPorts))
const knownProjectList = computed(() => filterByKeyword(portStore.knownProjectPorts))
const envServiceList = computed(() => filterByKeyword(portStore.envServicePorts))
const systemList = computed(() => filterByKeyword(portStore.systemPorts))
/** 已忽略区不参与搜索过滤（搜索时折叠区整体隐藏，见模板 v-if="!keyword"） */
const ignoredList = computed(() => portStore.ignoredPorts)

/** 统计条数据（搜索时按过滤后口径） */
const stats = computed(() => {
  const all = [
    ...focusList.value,
    ...knownProjectList.value,
    ...envServiceList.value,
    ...systemList.value,
  ]
  const counts = {} as Record<PortCategory, number>
  for (const p of all) counts[p.category] = (counts[p.category] ?? 0) + 1
  return counts
})

/** 某行是否提供「结束进程」按钮（系统/环境服务/托管项目不提供） */
function killable(p: ListeningPortInfo): boolean {
  return (
    p.category === 'suspicious' ||
    p.category === 'dev' ||
    p.category === 'other' ||
    p.category === 'known_project_external'
  )
}

/** 二次确认后结束整棵进程树 */
function confirmKill(p: ListeningPortInfo) {
  if (!killable(p)) return
  dialog.warning({
    title: '结束进程',
    content: `将强制结束「${p.display_title || p.process_name}」（PID ${p.owner_pid}）及其整棵进程树，端口 ${p.port} 随之释放。确定继续？`,
    positiveText: '结束进程',
    negativeText: '取消',
    onPositiveClick: async () => {
      const [result, error] = await portStore.killPort(p)
      if (error || !result) {
        message.error(error ?? '结束进程失败')
        return
      }
      if (result.failed > 0) {
        message.warning(`结束完成：成功 ${result.killed}，失败 ${result.failed}`)
      } else if (result.skipped_locked > 0) {
        // 单树调用，命中锁定表即整树跳过（killed 为 0）
        message.warning('目标进程已被内存清理器锁定，已跳过结束；如需结束请先在内存清理中解锁')
      } else {
        message.success(`已结束 ${result.killed} 棵进程树，释放 ${formatBytes(result.freed_bytes)}`)
      }
    },
  })
}

/** 分类标签（小号彩色） */
function categoryTag(p: ListeningPortInfo) {
  return PORT_CATEGORY_META[p.category]
}

/** 忽略可疑端口（无需二次确认——低风险且可撤销），成功后移入「已忽略」区 */
async function ignoreAction(p: ListeningPortInfo) {
  const ok = await portStore.ignorePort(p)
  if (!ok) {
    message.error(`忽略端口 ${p.port} 失败`)
    return
  }
  message.success(`已忽略端口 ${p.port}，不再计入提醒`, { duration: 2500 })
}

/** 恢复被忽略的端口，重新参与分类评分（若仍可疑则重新计入角标） */
async function unignoreAction(p: ListeningPortInfo) {
  const ok = await portStore.unignorePort(p)
  if (!ok) {
    message.error(`恢复端口 ${p.port} 失败`)
    return
  }
  message.success(`端口 ${p.port} 已恢复提醒`, { duration: 2500 })
}
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    title="端口监控"
    class="port-modal"
    style="width: 880px; max-width: 94vw"
    :content-style="{ display: 'flex', flexDirection: 'column', maxHeight: 'calc(80vh - 72px)' }"
    :mask-closable="true"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <!-- 顶部：分类统计条 -->
    <div class="stats-bar">
      <div class="stat-chip danger">
        <span class="stat-num">{{ stats.suspicious ?? 0 }}</span>
        <span class="stat-label">可疑</span>
      </div>
      <div class="stat-chip">
        <span class="stat-num">{{ stats.dev ?? 0 }}</span>
        <span class="stat-label">开发</span>
      </div>
      <div class="stat-chip">
        <span class="stat-num">
          {{ (stats.known_project ?? 0) + (stats.known_project_external ?? 0) }}
        </span>
        <span class="stat-label">已配置</span>
      </div>
      <div class="stat-chip">
        <span class="stat-num">{{ stats.env_service ?? 0 }}</span>
        <span class="stat-label">服务</span>
      </div>
      <div class="stat-chip">
        <span class="stat-num">{{ stats.system ?? 0 }}</span>
        <span class="stat-label">系统</span>
      </div>

      <div class="toolbar-gap" />
      <NInput
        v-model:value="keyword"
        size="small"
        placeholder="搜索端口 / 进程 / 路径"
        clearable
        style="width: 220px"
      >
        <template #prefix>
          <NIcon :component="SearchOutline" />
        </template>
      </NInput>
      <NButton size="small" quaternary :loading="portStore.scanning" @click="portStore.scan()">
        <template #icon>
          <NIcon><RefreshOutline /></NIcon>
        </template>
        刷新
      </NButton>
    </div>

    <!-- 提示：可疑端口的含义 -->
    <div v-if="(stats.suspicious ?? 0) > 0" class="hint-bar">
      <NIcon size="14"><WarningOutline /></NIcon>
      检测到 {{ stats.suspicious }} 个可疑端口：开发进程监听非项目配置端口
      （常见于 AI 工具 / 终端启动后遗留的测试服务），建议确认后结束。
    </div>

    <!-- 列表滚动区：统计条/提示条固定，列表超出后在弹窗内部滚动 -->
    <NSpin :show="portStore.scanning && portStore.ports.length === 0" class="scroll-area">
      <!-- 重点区：可疑 + 开发 + 其他 -->
      <div v-if="focusList.length" class="port-list">
        <div v-for="p in focusList" :key="`f-${p.port}`" class="port-row" :class="{ suspicious: p.category === 'suspicious' }">
          <span class="port-num">{{ p.port }}</span>
          <div class="port-main">
            <div class="port-title-line">
              <span class="port-title">{{ p.display_title || p.process_name }}</span>
              <NTag size="tiny" :color="{ color: 'transparent', textColor: categoryTag(p).color, borderColor: categoryTag(p).color }">
                {{ categoryTag(p).label }}
              </NTag>
              <NTag v-if="p.listen_all_interfaces" size="tiny" type="warning" :bordered="false">
                对外暴露
              </NTag>
              <NTag v-if="p.orphan" size="tiny" type="error" :bordered="false">孤立进程</NTag>
              <NTag v-if="p.random_port" size="tiny" type="warning" :bordered="false">随机端口</NTag>
              <NTag
                v-if="p.matched_project_name"
                size="tiny"
                :bordered="false"
                :type="p.matched_project_managed ? 'success' : 'info'"
              >
                {{ p.matched_project_name }}{{ p.matched_project_managed ? '' : '·外部' }}
              </NTag>
            </div>
            <div class="port-sub-line">
              <span v-if="p.project_path" class="sub-path" :title="p.project_path">{{ p.project_path }}</span>
              <span v-if="p.cmdline_summary" class="sub-cmd" :title="p.cmdline_summary">{{ p.cmdline_summary }}</span>
              <span class="sub-addr">{{ p.addresses.join(' / ') }}</span>
            </div>
          </div>
          <div class="port-meta">
            <div class="meta-time" :title="p.started_at ?? ''">{{ formatRelativeTime(p.started_at) || '未知' }}</div>
            <div class="meta-mem">{{ p.memory_bytes > 0 ? formatBytes(p.memory_bytes) : '' }}</div>
          </div>
          <div class="row-actions">
            <NButton
              v-if="killable(p)"
              size="tiny"
              type="error"
              secondary
              :loading="portStore.killing"
              @click="confirmKill(p)"
            >
              <template #icon>
                <NIcon><TrashOutline /></NIcon>
              </template>
              结束
            </NButton>
            <NButton
              v-if="p.category === 'suspicious'"
              size="tiny"
              quaternary
              @click="ignoreAction(p)"
            >
              <template #icon>
                <NIcon><EyeOffOutline /></NIcon>
              </template>
              忽略
            </NButton>
          </div>
        </div>
      </div>
      <NEmpty v-else-if="!keyword" description="暂无需要关注的端口" style="padding: 24px 0" />

      <!-- 折叠区 -->
      <template v-if="!keyword">
        <div v-if="knownProjectList.length" class="section-head" @click="collapsed.knownProject = !collapsed.knownProject">
          <NIcon size="14">
            <ChevronDownOutline v-if="!collapsed.knownProject" />
            <ChevronForwardOutline v-else />
          </NIcon>
          已配置项目端口（{{ knownProjectList.length }}）
        </div>
        <div v-if="!collapsed.knownProject" class="port-list compact">
          <div v-for="p in knownProjectList" :key="`k-${p.port}`" class="port-row">
            <span class="port-num">{{ p.port }}</span>
            <div class="port-main">
              <div class="port-title-line">
                <span class="port-title">{{ p.display_title || p.process_name }}</span>
                <NTag size="tiny" :bordered="false" :type="p.matched_project_managed ? 'success' : 'info'">
                  {{ p.matched_project_name }}{{ p.matched_project_managed ? '' : '·外部启动' }}
                </NTag>
              </div>
              <div class="port-sub-line">
                <span class="sub-addr">{{ p.addresses.join(' / ') }}</span>
              </div>
            </div>
            <NButton
              v-if="killable(p)"
              size="tiny"
              quaternary
              type="error"
              :loading="portStore.killing"
              @click="confirmKill(p)"
            >
              结束
            </NButton>
          </div>
        </div>

        <div v-if="envServiceList.length" class="section-head" @click="collapsed.envService = !collapsed.envService">
          <NIcon size="14">
            <ChevronDownOutline v-if="!collapsed.envService" />
            <ChevronForwardOutline v-else />
          </NIcon>
          环境服务（{{ envServiceList.length }}）
        </div>
        <div v-if="!collapsed.envService" class="port-list compact">
          <div v-for="p in envServiceList" :key="`e-${p.port}`" class="port-row">
            <span class="port-num">{{ p.port }}</span>
            <div class="port-main">
              <div class="port-title-line">
                <span class="port-title">{{ p.display_title || p.process_name }}</span>
              </div>
              <div class="port-sub-line">
                <span class="sub-addr">{{ p.addresses.join(' / ') }}</span>
              </div>
            </div>
          </div>
        </div>

        <div v-if="systemList.length" class="section-head" @click="collapsed.system = !collapsed.system">
          <NIcon size="14">
            <ChevronDownOutline v-if="!collapsed.system" />
            <ChevronForwardOutline v-else />
          </NIcon>
          系统端口（{{ systemList.length }}）
        </div>
        <div v-if="!collapsed.system" class="port-list compact">
          <div v-for="p in systemList" :key="`s-${p.port}`" class="port-row">
            <span class="port-num">{{ p.port }}</span>
            <div class="port-main">
              <div class="port-title-line">
                <span class="port-title">{{ p.process_name || '系统内核' }}</span>
              </div>
              <div class="port-sub-line">
                <span class="sub-addr">{{ p.addresses.join(' / ') }}</span>
              </div>
            </div>
          </div>
        </div>

        <div v-if="ignoredList.length" class="section-head" @click="collapsed.ignored = !collapsed.ignored">
          <NIcon size="14">
            <ChevronDownOutline v-if="!collapsed.ignored" />
            <ChevronForwardOutline v-else />
          </NIcon>
          已忽略（{{ ignoredList.length }}）
        </div>
        <div v-if="!collapsed.ignored" class="port-list compact">
          <div v-for="p in ignoredList" :key="`i-${p.port}`" class="port-row ignored">
            <span class="port-num">{{ p.port }}</span>
            <div class="port-main">
              <div class="port-title-line">
                <span class="port-title">{{ p.display_title || p.process_name }}</span>
                <NTag size="tiny" :bordered="false" :color="{ color: 'transparent', textColor: '#8c8c8c', borderColor: '#d9d9d9' }">
                  已忽略
                </NTag>
              </div>
              <div class="port-sub-line">
                <span class="sub-addr">{{ p.addresses.join(' / ') }}</span>
              </div>
            </div>
            <NButton size="tiny" quaternary @click="unignoreAction(p)">恢复</NButton>
          </div>
        </div>
      </template>
    </NSpin>
  </NModal>
</template>

<style scoped>
/* 弹窗高度约束的配套：content 区为 flex 列，统计条/提示条固定，
   列表区（NSpin 及以下）作为唯一滚动容器 */
.stats-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 10px;
  flex-shrink: 0;
}
.scroll-area {
  /* basis 必须为 auto：容器高度是 max-height（非固定值），basis 0 会把
     本区高度塌陷为 0（auto 容器没有多余空间可供 grow 分配）；
     内容不足时弹窗自适应矮，超出时由 min-height:0 + shrink 收缩出滚动 */
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  padding-right: 2px;
}
.row-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.toolbar-gap {
  flex: 1;
}
.stat-chip {
  display: flex;
  align-items: baseline;
  gap: 4px;
  padding: 2px 10px;
  border-radius: 10px;
  background: var(--toolbar-bg, #fafafa);
  border: 1px solid var(--divider);
}
.stat-chip .stat-num {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  font-family: var(--code-font);
}
.stat-chip .stat-label {
  font-size: 12px;
  color: var(--text-tertiary);
}
.stat-chip.danger .stat-num {
  color: #f5222d;
}
.stat-chip.danger {
  border-color: rgba(245, 34, 45, 0.4);
  background: rgba(245, 34, 45, 0.06);
}

.hint-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  margin-bottom: 10px;
  border-radius: 4px;
  font-size: 12px;
  color: #d4380d;
  background: rgba(245, 34, 45, 0.06);
  border: 1px solid rgba(245, 34, 45, 0.25);
  flex-shrink: 0;
}

.port-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.port-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border-radius: 6px;
  border: 1px solid var(--divider);
  background: var(--card-bg);
}
.port-row.suspicious {
  border-color: rgba(245, 34, 45, 0.35);
  background: rgba(245, 34, 45, 0.03);
}
.port-row .port-num {
  min-width: 56px;
  font-family: var(--code-font);
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  text-align: right;
}
.port-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.port-title-line {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.port-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}
.port-sub-line {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  font-size: 12px;
  color: var(--text-tertiary);
}
.sub-path {
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sub-cmd {
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--code-font);
  font-size: 11px;
}
.sub-addr {
  font-family: var(--code-font);
  font-size: 11px;
}
.port-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  flex-shrink: 0;
  min-width: 70px;
}
.meta-time {
  font-size: 12px;
  color: var(--text-secondary);
}
.meta-mem {
  font-size: 11px;
  color: var(--text-tertiary);
  font-family: var(--code-font);
}

.section-head {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 4px 4px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}
.section-head:hover {
  color: var(--accent);
}
.port-list.compact .port-row {
  padding: 4px 10px;
}
.port-list.compact .port-num {
  font-size: 14px;
  min-width: 56px;
}
.port-row.ignored {
  opacity: 0.75;
}
.port-row.ignored .port-num {
  color: var(--text-tertiary);
}
</style>
