<script setup lang="ts">
// 项目新增/编辑表单 Dialog。
//
// 设计：
// - create / edit 复用同一表单，edit 模式预填 project 字段
// - 提交时构造 ProjectInput（snake_case 对齐 Rust serde）抛给父组件
// - 目录选择用前端 dialog 插件 open({directory:true})（dialog:default 已含 allow-open）
//   —— 比 Rust pick_directory 命令更简单，无需新增后端代码
// - 端口输入用逗号分隔字符串，提交时拆分数组并 trim
//
// 表单不做 JSON 字段，只做 UI 层校验；后端有 DB 约束兜底（如 name UNIQUE）。

import { computed, reactive, ref, watch } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import {
  NModal,
  NForm,
  NFormItem,
  NInput,
  NInputGroup,
  NSelect,
  NSwitch,
  NButton,
  type FormInst,
  type FormRules,
} from 'naive-ui'
import {
  PROJECT_TYPE_OPTIONS,
  type Project,
  type ProjectInput,
  type ProjectType,
} from '@/types/project'
import { parsePorts } from '@/utils/ports'

const props = defineProps<{
  modelValue: boolean
  /** 编辑模式时传入原项目；新建模式传 null */
  project?: Project | null
  /** 提交中 */
  submitting?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  /** 提交，payload 为 ProjectInput（edit 模式由父组件负责带 id 调 update） */
  submit: [input: ProjectInput, origin: Project | null]
}>()

// ===== 表单模型 =====

interface FormState {
  name: string
  type: ProjectType
  path: string
  /** 运行时工作目录（空=用 path） */
  workdir: string
  /** 扫描根目录（空=不归属任何扫描目录面板） */
  scan_root: string
  start_cmd: string
  build_cmd: string
  expected_ports: string // 输入用逗号分隔，提交时拆数组
  enabled: boolean
}

const EMPTY_FORM: FormState = {
  name: '',
  type: 'custom',
  path: '',
  workdir: '',
  scan_root: '',
  start_cmd: '',
  build_cmd: '',
  expected_ports: '',
  enabled: true,
}

const form = reactive<FormState>({ ...EMPTY_FORM })
const formRef = ref<FormInst | null>(null)

const isEdit = computed(() => !!props.project)
const title = computed(() => (isEdit.value ? '编辑项目' : '新建项目'))

const visible = computed({
  get: () => props.modelValue,
  set: (v: boolean) => emit('update:modelValue', v),
})

// dialog 打开时同步表单
watch(
  () => props.modelValue,
  (open) => {
    if (!open) return
    if (props.project) {
      form.name = props.project.name
      form.type = props.project.type
      form.path = props.project.path
      form.workdir = props.project.workdir ?? ''
      form.scan_root = props.project.scan_root ?? ''
      form.start_cmd = props.project.start_cmd
      form.build_cmd = props.project.build_cmd ?? ''
      form.expected_ports = props.project.expected_ports.join(', ')
      form.enabled = props.project.enabled
    } else {
      Object.assign(form, { ...EMPTY_FORM })
    }
    formRef.value?.restoreValidation()
  },
)

// ===== 校验 =====

const rules: FormRules = {
  name: [{ required: true, message: '请输入项目名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择项目类型', trigger: 'change' }],
  path: [{ required: true, message: '请选择项目目录', trigger: 'change' }],
  start_cmd: [{ required: true, message: '请输入启动命令', trigger: 'blur' }],
}

// ===== 目录选择 =====

async function pickDirectory() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: '选择项目目录',
      defaultPath: form.path || undefined,
    })
    if (typeof selected === 'string' && selected.length > 0) {
      form.path = selected
      // 清除 path 字段可能的「请选择项目目录」校验错误
      formRef.value?.restoreValidation()
    }
  } catch (e) {
    // 用户取消会 reject，忽略即可
    console.debug('pick directory canceled or failed:', e)
  }
}

async function pickWorkdir() {
  try {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: '选择运行时工作目录',
      defaultPath: form.workdir || form.path || undefined,
    })
    if (typeof selected === 'string' && selected.length > 0) {
      form.workdir = selected
    }
  } catch (e) {
    console.debug('pick workdir canceled or failed:', e)
  }
}

// ===== 提交 =====

/** 把 FormState 转成 Rust 端 ProjectInput（snake_case） */
function buildInput(): ProjectInput {
  const ports = parsePorts(form.expected_ports)
  return {
    name: form.name.trim(),
    type: form.type,
    path: form.path.trim(),
    workdir: form.workdir.trim() || null,
    scan_root: form.scan_root.trim() || null,
    start_cmd: form.start_cmd.trim(),
    build_cmd: form.build_cmd.trim() || null,
    expected_ports: ports,
    enabled: form.enabled,
  }
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate(async (errors) => {
    if (errors) return
    emit('submit', buildInput(), props.project ?? null)
  })
}

function handleClose() {
  visible.value = false
}
</script>

<template>
  <NModal
    v-model:show="visible"
    preset="card"
    :title="title"
    style="width: 560px"
    :mask-closable="false"
  >
    <NForm
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="88"
      label-placement="left"
      require-mark-placement="right-hanging"
    >
      <NFormItem label="项目名称" path="name">
        <NInput v-model:value="form.name" placeholder="如：HR后端" clearable />
      </NFormItem>

      <NFormItem label="项目类型" path="type">
        <NSelect v-model:value="form.type" :options="PROJECT_TYPE_OPTIONS" placeholder="选择类型" />
      </NFormItem>

      <NFormItem label="扫描目录" path="scan_root">
        <NInput
          v-model:value="form.scan_root"
          placeholder="扫描添加时自动填写；手动添加可留空"
          clearable
        />
      </NFormItem>

      <NFormItem label="项目目录" path="path">
        <NInputGroup>
          <NInput
            v-model:value="form.path"
            placeholder="点击右侧按钮选择目录"
            readonly
            style="flex: 1"
          />
          <NButton @click="pickDirectory">选择...</NButton>
        </NInputGroup>
      </NFormItem>

      <NFormItem label="工作目录" path="workdir">
        <NInputGroup>
          <NInput
            v-model:value="form.workdir"
            placeholder="留空则同项目目录；license 等资源在上级目录时填此项"
            readonly
            style="flex: 1"
          />
          <NButton @click="pickWorkdir">选择...</NButton>
        </NInputGroup>
      </NFormItem>

      <NFormItem label="启动命令" path="start_cmd">
        <NInput
          v-model:value="form.start_cmd"
          placeholder="如：npm run dev / mvn spring-boot:run"
          clearable
        />
      </NFormItem>

      <NFormItem label="构建命令" path="build_cmd">
        <NInput
          v-model:value="form.build_cmd"
          placeholder="可选，如：mvn clean package / npm run build"
          clearable
        />
      </NFormItem>

      <NFormItem label="预期端口" path="expected_ports">
        <NInput
          v-model:value="form.expected_ports"
          placeholder="多个端口用逗号分隔，如：8080, 5173"
          clearable
        />
      </NFormItem>

      <NFormItem label="启用">
        <NSwitch v-model:value="form.enabled" />
        <span class="hint">关闭后该项目不在列表执行批量操作</span>
      </NFormItem>
    </NForm>

    <template #footer>
      <div class="footer">
        <NButton @click="handleClose">取消</NButton>
        <NButton type="primary" :loading="props.submitting" @click="handleSubmit">
          {{ isEdit ? '保存' : '创建' }}
        </NButton>
      </div>
    </template>
  </NModal>
</template>

<style scoped>
.hint {
  margin-left: 8px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
