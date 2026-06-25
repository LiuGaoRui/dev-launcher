# CURRENT - 交接状态单

> **每个会话开头读此文件**，了解当前进度与下一步；**每个会话结束前更新此文件**。
> 最后更新：阶段 0 ✅ 完成

## 一、项目概况

- **项目**：DevLauncher — 开发机项目管理器（本地多项目一键启停 / 监控 / 日志）
- **技术栈**：Tauri 2.x + Vue 3 + Element Plus + Pinia + SQLite (tauri-plugin-sql)
- **范围**：仅 MVP（详见 `docs/开发计划.md`）

## 二、阶段进度

| 阶段 | 内容 | 状态 |
|---|---|---|
| 0 | 脚手架 + 文档骨架 | ✅ 完成 |
| 1 | DB 层（migration + service CRUD） | ⬜ 下一个 |
| 2 | 进程托管核心（spawn + Job Object + registry） | ⬜ 未开始 |
| 3 | 项目注册 UI（列表 + 表单 + 分组） | ⬜ 未开始 |
| 4 | 启停 UI + 命令对接 | ⬜ 未开始 |
| 5 | 监控面板（sysinfo + 端口探测） | ⬜ 未开始 |
| 6 | 日志流（实时 tail + 历史分页） | ⬜ 未开始 |
| 7 | 构建命令 + 一键发布 | ⬜ 未开始 |

## 三、当前状态

**阶段 0 已完成**。项目骨架可运行：
- ✅ 前端 Vite dev server 正常启动（localhost:1420）
- ✅ 前端 build 成功（dist/ 已生成）
- ✅ Rust 后端 cargo check 通过（仅有未使用字段的无害 warning）
- ✅ 所有文档骨架就位（架构/DDL/命令清单/目录规范/环境搭建 + 4 个 ADR + 阶段总结）

## 四、下一步 → 阶段 1（DB 层）

1. 写 `src-tauri/migrations/0001_init.sql`（DDL 来自 `docs/02-数据库设计.md`）
2. `src-tauri/src/db/migrations.rs`：注册 tauri-plugin-sql 的 Migration
3. `src-tauri/src/services/` 模块：GroupService + ProjectService 的 CRUD
4. `src-tauri/src/models.rs`：Group / Project 等 serde 结构体
5. 对应 commands 薄层（`commands/group.rs`, `commands/project.rs`）
6. 在 `lib.rs` 的 `run()` 中注册命令 + 填充 migrations()
7. Rust 单测验证 CRUD

## 五、关键决策备忘（详见 docs/adr/）

- **ADR-001 进程管理**：不用 `tauri-plugin-shell`，用 `tokio::process` + Windows Job Object 自管
- **ADR-002 日志推送**：用 `tauri::ipc::Channel`（不用事件系统），前端 GC Channel 自动退订
- **ADR-003 端口探测**：`netstat2` 查监听者 + `TcpStream::connect` 探活双重检测
- **ADR-004 数据库**：用 `tauri-plugin-sql`（SQLite），`include_str!` 嵌入迁移 SQL

## 六、已验证的环境

- Node: v22.22.0
- pnpm: 11.9.0
- Rust: 1.95（stable，x86_64-pc-windows-msvc）
- VS 2022 Professional + C++ NativeDesktop 工作负载（VC Tools 14.39 + Windows SDK）
- Cargo 镜像：rsproxy.cn（配置在 `~/.cargo/config.toml`）
- npm 镜像：npmmirror.com

## 七、阶段 0 遇到的环境问题（已解决）

1. **pnpm 11 ERR_PNPM_IGNORED_BUILDS 致命错误**
   - 解决：`pnpm-workspace.yaml` 配 `onlyBuiltDependencies`；手动跑 esbuild install.js

2. **Rust 链接失败：`link: extra operand`**
   - 根因：VS 2022 缺 C++ 工作负载，cargo 误用 Git Bash 的 GNU `link.exe`
   - 解决：VS Installer 安装 `Microsoft.VisualStudio.Workload.NativeDesktop`

3. **tauri-winres RC.EXE 编译资源失败**
   - 根因：占位图标是 PNG 数据伪装成 .ico，RC.EXE 校验失败
   - 解决：用 Node.js 生成真正有效的 ICO + PNG 图标

4. **generate_context! panic: frontendDist 不存在**
   - 解决：先 `vite build` 生成 dist/（正常 `tauri dev` 会自动处理，`cargo check` 需要 dist 存在）

5. **cargo 编译缺 `use tauri::Manager`**
   - 解决：lib.rs 添加 `use tauri::Manager;`

## 八、文件清单（阶段 0 产出）

详见 `docs/progress/00-scaffold.md`
