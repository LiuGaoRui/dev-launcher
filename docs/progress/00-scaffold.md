# 阶段 0 - 脚手架完成总结

> 完成：项目骨架可运行（前端 Vite dev + Rust cargo check）

## 产出物

### 文档（docs/）
- `01-架构设计.md` — 整体架构、分层、数据流
- `02-数据库设计.md` — SQLite DDL 全文 + 索引策略
- `03-命令清单.md` — 18 个 Tauri Command 签名 + TS 类型
- `10-目录结构规范.md` — 前后端目录约定
- `20-环境搭建.md` — 从零搭建开发环境
- `progress/CURRENT.md` — 跨会话交接单

### 前端脚手架
- `package.json` — Vue 3 + Element Plus + Pinia + Vue Router + Tauri API
- `vite.config.ts` — 端口 1420（Tauri 约定）、Element Plus 自动导入、@ 别名
- `tsconfig.json` / `tsconfig.node.json` — strict mode
- `index.html` — 入口
- `src/main.ts` — 应用装配（Pinia + Router + Element Plus 中文）
- `src/App.vue` — 侧边栏 + 主区域布局
- `src/style.css` — 全局样式
- `src/router/index.ts` — 3 个路由（projects / projects/:id / groups）
- `src/views/` — 3 个占位页面

### Rust 后端脚手架（src-tauri/）
- `Cargo.toml` — tauri 2 + tauri-plugin-dialog + tauri-plugin-sql + tokio + sysinfo + netstat2 + windows
- `tauri.conf.json` — 窗口 1280x800、MSI/NSIS bundle
- `capabilities/default.json` — core + sql + dialog 权限
- `src/main.rs` — windows_subsystem + 调 lib::run()
- `src/lib.rs` — tauri::Builder 装配（dialog + sql plugin + tracing）
- `src/error.rs` — AppError + AppResult（thiserror + serde）
- `src/state.rs` — AppState（data_dir / logs_root）

## 验证结果

- ✅ pnpm 11.9 安装成功
- ✅ 前端依赖安装成功
- ✅ Vite dev server 在 localhost:1420 正常启动
- ✅ Vite build 成功，dist/ 已生成
- ✅ Rust 后端 cargo check 通过（仅有未使用字段的无害 warning）

## 遇到的问题与解决

### 1. pnpm 11 `ERR_PNPM_IGNORED_BUILDS` 致命错误
- **现象**：esbuild/vue-demi 的 build script 被忽略，导致 `pnpm install` 退出码 1，连带 `pnpm dev`（preinstall 检查）也失败
- **解决**：在 `pnpm-workspace.yaml` 配置 `onlyBuiltDependencies`，手动运行 esbuild 的 install.js
- **后续**：如再次遇到，运行 `node node_modules/.pnpm/esbuild@*/node_modules/esbuild/install.js`

### 2. Rust 链接失败：`link: extra operand`（GNU link.exe 被误用）
- **根因**：VS 2022 Professional 已装但缺 C++ 工作负载，无 MSVC `link.exe`；cargo 在 PATH 中找到 Git Bash 的 GNU coreutils `link.exe`（功能完全不同）
- **解决**：VS Installer 安装 `Microsoft.VisualStudio.Workload.NativeDesktop`（VC Tools + Windows SDK）

### 3. tauri-winres RC.EXE 编译资源失败
- **根因**：占位图标是 1x1 PNG 数据伪装成 .ico，Windows 资源编译器校验 ICO 格式失败
- **解决**：用 Node.js 生成真正有效的 ICO（含正确 ICO header + directory + PNG 位图）+ 多尺寸 PNG

### 4. generate_context! panic: frontendDist "../dist" 不存在
- **根因**：`tauri::generate_context!()` 宏在编译期检查 `frontendDist` 目录存在性
- **解决**：先 `vite build` 生成 dist/。正常 `pnpm tauri dev` 会自动通过 `beforeDevCommand` 处理，裸 `cargo check` 需要 dist 存在

### 5. cargo 缺 `use tauri::Manager`
- **解决**：lib.rs 添加 `use tauri::Manager;`（`.manage()` 方法需要此 trait）

## 下一步 → 阶段 1（DB 层）

1. 写 `src-tauri/migrations/0001_init.sql`（DDL 来自 02-数据库设计.md）
2. `src-tauri/src/db/` 模块：migration 注册 + 连接管理
3. `src-tauri/src/services/` 模块：GroupService + ProjectService 的 CRUD
4. 对应 commands 薄层
5. Rust 单测验证 CRUD
