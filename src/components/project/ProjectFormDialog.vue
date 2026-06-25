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
import type { FormInstance, FormRules } from 'element-plus'
import {
  PROJECT_TYPE_OPTIONS,
  type Project,
  type ProjectInput,
  type ProjectType,
} from '@/types/project'
import type { Group } from '@/types/group'

const props = defineProps<{
  modelValue: boolean
  /** 编辑模式时传入原项目；新建模式传 null */
  project?: Project | null
  /** 可选分组列表（供下拉） */
  groups: Group[]
  /** 新建模式下的默认分组（如在分组 tab 内点新建时预选） */
  defaultGroupId?: number | null
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
  group_id: number | null
  type: ProjectType
  path: string
  start_cmd: string
  build_cmd: string
  expected_ports: string // 输入用逗号分隔，提交时拆数组
  enabled: boolean
}

const EMPTY_FORM: FormState = {
  name: '',
  group_id: null,
  type: 'custom',
  path: '',
  start_cmd: '',
  build_cmd: '',
  expected_ports: '',
  enabled: true,
}

const form = reactive<FormState>({ ...EMPTY_FORM })
const formRef = ref<FormInstance>()

const isEdit = computed(() => !!props.project)
const title = computed(() => (isEdit.value ? '编辑项目' : '新建项目'))

// dialog 打开时同步表单
watch(
  () => props.modelValue,
  (open) => {
    if (!open) return
    if (props.project) {
      form.name = props.project.name
      form.group_id = props.project.group_id
      form.type = props.project.type
      form.path = props.project.path
      form.start_cmd = props.project.start_cmd
      form.build_cmd = props.project.build_cmd ?? ''
      form.expected_ports = props.project.expected_ports.join(', ')
      form.enabled = props.project.enabled
    } else {
      Object.assign(form, { ...EMPTY_FORM })
      // 应用默认分组（如在分组 tab 内点新建）
      form.group_id = props.defaultGroupId ?? null
    }
    formRef.value?.clearValidate()
  },
)

// ===== 校验 =====

const rules: FormRules<FormState> = {
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
      formRef.value?.validateField('path')
    }
  } catch (e) {
    // 用户取消会 reject，忽略即可
    console.debug('pick directory canceled or failed:', e)
  }
}

// ===== 提交 =====

/** 把 FormState 转成 Rust 端 ProjectInput（snake_case） */
function buildInput(): ProjectInput {
  const ports = form.expected_ports
    .split(/[,，\s]+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
  return {
    name: form.name.trim(),
    group_id: form.group_id,
    type: form.type,
    path: form.path.trim(),
    start_cmd: form.start_cmd.trim(),
    build_cmd: form.build_cmd.trim() || null,
    expected_ports: ports,
    enabled: form.enabled,
  }
}

async function handleSubmit() {
  if (!formRef.value) return
  const valid = await formRef.value.validate().catch(() => false)
  if (!valid) return
  emit('submit', buildInput(), props.project ?? null)
}

function handleClose() {
  emit('update:modelValue', false)
}
</script>

<template>
  <el-dialog
    :model-value="props.modelValue"
    :title="title"
    width="560px"
    :close-on-click-modal="false"
    append-to-body
    @update:model-value="handleClose"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="88px"
      label-position="right"
    >
      <el-form-item label="项目名称" prop="name">
        <el-input v-model="form.name" placeholder="如：HR后端" clearable />
      </el-form-item>

      <el-form-item label="项目类型" prop="type">
        <el-select v-model="form.type" placeholder="选择类型" style="width: 100%">
          <el-option
            v-for="opt in PROJECT_TYPE_OPTIONS"
            :key="opt.value"
            :label="opt.label"
            :value="opt.value"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="所属分组" prop="group_id">
        <el-select
          v-model="form.group_id"
          placeholder="不分组"
          clearable
          style="width: 100%"
        >
          <el-option
            v-for="g in props.groups"
            :key="g.id"
            :label="g.name"
            :value="g.id"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="项目目录" prop="path">
        <el-input v-model="form.path" placeholder="点击右侧按钮选择目录" readonly>
          <template #append>
            <el-button @click="pickDirectory">选择...</el-button>
          </template>
        </el-input>
      </el-form-item>

      <el-form-item label="启动命令" prop="start_cmd">
        <el-input
          v-model="form.start_cmd"
          placeholder="如：npm run dev / mvn spring-boot:run"
          clearable
        />
      </el-form-item>

      <el-form-item label="构建命令" prop="build_cmd">
        <el-input
          v-model="form.build_cmd"
          placeholder="可选，如：mvn clean package / npm run build"
          clearable
        />
      </el-form-item>

      <el-form-item label="预期端口" prop="expected_ports">
        <el-input
          v-model="form.expected_ports"
          placeholder="多个端口用逗号分隔，如：8080, 5173"
          clearable
        />
      </el-form-item>

      <el-form-item label="启用">
        <el-switch v-model="form.enabled" />
        <span class="hint">关闭后该项目不在列表执行批量操作</span>
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="handleClose">取消</el-button>
      <el-button
        type="primary"
        :loading="props.submitting"
        @click="handleSubmit"
      >
        {{ isEdit ? '保存' : '创建' }}
      </el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.hint {
  margin-left: 8px;
  font-size: 12px;
  color: #909399;
}
</style>
