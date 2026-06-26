// 与 src-tauri/src/models.rs 的 DetectedProject / LaunchScheme 对齐。
// 仅用于 scan_projects 命令返回值（Rust→前端展示），用户勾选后转 ProjectInput 入库。

import type { ProjectType } from './project'

/** 启动方案：一对 (start_cmd, build_cmd) + 说明 */
export interface LaunchScheme {
  /** 方案名，如 "开发模式" / "打包运行模式" */
  label: string
  /** 是否推荐方案（前端默认选中） */
  recommended: boolean
  /** 启动命令 */
  start_cmd: string
  /** 构建命令（可选） */
  build_cmd: string | null
  /** 方案说明（tooltip / 占位提示） */
  description: string
}

/** 扫描检测到的项目（仅 Rust→前端，不入库） */
export interface DetectedProject {
  /** 相对根目录的展示路径，如 "backend/hr-service" */
  rel_path: string
  /** 绝对路径（→ project.path，找 pom/package.json 的目录） */
  path: string
  /** 运行时工作目录（→ project.workdir，默认=扫描根目录） */
  workdir: string
  /** 推断的项目名 */
  name: string
  /** 项目类型（springboot | java_jar | node） */
  type: ProjectType
  /** 推断的端口列表 */
  expected_ports: string[]
  /** 候选启动方案 */
  schemes: LaunchScheme[]
}
