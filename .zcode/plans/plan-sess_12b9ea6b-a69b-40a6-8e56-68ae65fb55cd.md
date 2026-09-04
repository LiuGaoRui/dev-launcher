# 端口监控弹窗优化：高度约束 + 可疑端口忽略

## 需求
1. 弹窗总高度最多占窗口 80%，滚动条放在弹窗内部（统计条/搜索框保持可见）
2. 可疑端口行增加「忽略」按钮：忽略后不再提示、不计入角标，且可恢复

## 优化 1：高度控制 + 内部滚动（纯前端，1 个文件）

`src/components/port/PortMonitorModal.vue`：
- NModal（preset card）加 `content-style="display: flex; flex-direction: column; max-height: calc(80vh - 56px)"`——80vh 为窗口 80%，56px 为卡片标题栏高度
- 统计条 + 可疑提示条（stats-bar / hint-bar）固定顶部不随滚动
- 列表区（NSpin 及以下）作为唯一滚动容器：`flex: 1; min-height: 0; overflow-y: auto`

## 优化 2：可疑端口忽略（后端持久化，对齐 cleaner_lock 模式）

### 设计决策
- **匹配键 = 端口号**：端口是稳定标识，进程重启 PID 变化不影响；同端口被其他进程占用仍被压制（语义 =「这个端口号我认可」，与 cleaner 的进程级锁定互补）
- **持久化到 SQLite** 新表 `port_ignore`（遵循 cleaner_lock 先例；项目已有 localStorage 锁定被废弃的教训）
- **分类处理**：`PortCategory` 新增 `Ignored` 档；`scan_listening_ports` 在单端口分类完成后，命中忽略表则统一覆盖为 `Ignored / score 0`（用户显式意志优先于所有推断）。suspiciousCount 只统计 suspicious → 角标自动不再计数，无需改 TitleBar
- **UI 交互**：仅 suspicious 行显示「忽略」按钮（低风险可撤销，不二次确认，message 提示）；新增「已忽略（N）」默认折叠区，行内提供「恢复」按钮；已忽略行不提供「结束」按钮（防误杀，同系统端口思路）
- **不做 list 命令**：被忽略端口监听时以 Ignored 分类出现在扫描结果中即可见可恢复；未监听的死条目无害（该端口再被占用时会重新出现）

### 改动文件（后端 7 + 前端 4，共 11 个）

| 文件 | 改动 |
|---|---|
| `src-tauri/migrations/0006_add_port_ignore.sql`（新） | `port_ignore(id, port UNIQUE, create_time)` |
| `src-tauri/src/db/migrations.rs` | 注册 version 6 |
| `src-tauri/src/services/port_ignore_service.rs`（新） | `add(port)` / `remove(port)` / `list_ignored() -> HashSet<u16>`，含单测 |
| `src-tauri/src/services/mod.rs` | 注册模块 |
| `src-tauri/src/process/port_scan.rs` | `PortCategory::Ignored`；`scan_listening_ports` 加 `ignored_ports: &HashSet<u16>` 参数，循环内覆盖 category/score；单测（注入真实进程验证覆盖） |
| `src-tauri/src/commands/port.rs` | `list_listening_ports` 查忽略表传参（不访问 DB 的分层不变）；新增薄命令 `add_port_ignore` / `remove_port_ignore` |
| `src-tauri/src/lib.rs` | 注册 2 个新命令 |
| `src/types/port.ts` | `'ignored'` 类型 + PORT_CATEGORY_META 条目（灰色「已忽略」） |
| `src/api/port.ts` | `addPortIgnore(port)` / `removePortIgnore(port)` |
| `src/stores/port.ts` | `ignoredPorts` computed；`ignorePort` / `unignorePort`（成功后 rescan） |
| `src/components/port/PortMonitorModal.vue` | 滚动布局 + 可疑行忽略按钮 + 已忽略折叠区/恢复按钮 |

### 风险与权衡
- 按端口号忽略的固有语义：换进程占用同端口仍被压制——可解释、无害（端口再被可疑进程占用时仍不想被提醒，正是忽略意图）
- `Ignored` score=0 与 System 并列：无实际影响（Ignored 只进独立折叠区，不参与重点区排序展示）

## 验证
1. `cargo test`（service 单测 + port_scan 忽略覆盖单测，全量回归）
2. `pnpm typecheck`
3. Playwright mock IPC 冒烟：弹窗内容区出现内部滚动且总高 ≤ 80vh；点忽略后该端口移入「已忽略」折叠区、角标数减少；恢复后回到可疑区、角标恢复