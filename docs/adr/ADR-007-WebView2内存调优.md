# ADR-007: WebView2 内存调优

**状态**：已采纳
**日期**：2026-09-14

## 背景

用户反馈应用刚启动、无任何操作时内存占用即偏高。DevLauncher 是常驻挂机的
开发机管理器，WebView2 的内存表现直接影响整机的可用内存。

代码调查确认三个事实：

1. **基线无任何调优**：单窗口单 webview（tauri 2.11.3 / wry 0.55.1），未设置
   `additionalBrowserArgs`，基线即 WebView2 多进程（browser / GPU / renderer）默认开销。
2. **日志缓冲无界增长**：`useLogStore` 的 `lines.value += chunk.text` 无上限，后端
   每 250ms 推送且单块不限大小，全量渲染进 `<pre>`，且每次推送触发 O(n) 行数全扫。
   这是唯一「越用越高」的源头。
3. **wry 的 `additionalBrowserArgs` 是完全替换语义**（源码 `unwrap_or_else`）：
   自定义时必须手动补回 wry 默认的
   `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`，
   否则右键菜单 / SmartScreen 行为回归。

## 决策

三个杠杆组合，验收标准为「稳态有界」：

1. **浏览器参数压前台基线**（`tauri.conf.json` 一行）：
   `--disable-gpu`（GPU 进程转软合成，省约 30~60MB）+
   `--js-flags=--max-old-space-size=128`（renderer V8 old space 封顶），
   并补回 wry 默认 disable-features 段。
2. **失焦降内存**（`webview_tuning.rs`）：监听 `WindowEvent::Focused`，失焦时把
   WebView2 的 MemoryUsageTargetLevel 设为 Low（部分浏览器进程内存换出磁盘，
   JS 继续运行，轮询与日志订阅不受影响），聚焦恢复 Normal。
3. **日志缓冲有界**（`stores/log.ts`）：双上限截断头部——超过 5000 行丢整行，
   另设 5M 字符硬上限兜底无换行的病态输出（如 `\r` 进度条、单行压缩文本，
   行数截断对其无效，且无界增长会撞 128MB old space 上限导致 renderer OOM）。
   行数增量维护，不再全扫。全量日志仍在磁盘 `{logs_root}/{project_id}/{start|build}.log`，
   UI 截断不丢数据。

## 理由

- **MemoryUsageTargetLevel 是微软官方推荐的应用失活降内存途径**（WebView2
  Runtime >= 114.0.1823.32，2023 年中版本，现网覆盖率近乎 100%）。wry 0.35+
  已在内部实现（ICoreWebView2_19），应用层经 `with_webview` 取 controller 后
  cast 调用即可，约 40 行、无新增 heavyweight 依赖（webview2-com 与 wry 同版本对齐）。
- **失焦即 Low 比仅最小化激进**：本应用挂机为主，大部分时间不聚焦；切换调用
  幂等且轻量，聚焦恢复无明显代价（换出内存按需读回）。
- **V8 上限取 128MB**：Vue3 + Naive UI + 5000 行日志稳态估计 50~80MB，128 留余量，
  与日志截断配合后 OOM（renderer 白屏）概率低。
- **`--disable-gpu` 段可独立摘除**：若实测滚动 / 动画掉帧不可接受，删掉该参数
  不影响其余两杠杆。

## 代价

- 前台动画 / 滚动走 CPU 软合成（本项目 UI 动画少，预期可接受；不可接受则摘参数）。
- 日志缓冲双上限（5000 行 / 5M 字符 ≈ 10MB UTF-16）后，缓冲本身不再可能撑爆
  128MB old space，OOM 白屏风险基本消除；极端场景仅丢更早的日志展示。
- 旧 WebView2 运行时（< 114）上失焦降内存静默无效，无副作用。
- UI 只能看最近 5000 行日志，更早历史需查看磁盘文件。

## 实现

```json
// tauri.conf.json
"additionalBrowserArgs": "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-gpu --js-flags=--max-old-space-size=128"
```

```rust
// lib.rs — 失焦/聚焦事件挂接
.on_window_event(|window, event| {
    if let tauri::WindowEvent::Focused(focused) = event {
        webview_tuning::on_focus_change(window, *focused);
    }
})
```

```rust
// webview_tuning.rs — ICoreWebView2_19::SetMemoryUsageTargetLevel(Low/Normal)
// cast 失败（运行时过旧）静默跳过
```

```typescript
// stores/log.ts — 有界缓冲
const MAX_LINES = 5000
// appendChunk: 追加 → 超 5000 行丢头部整行 → 增量维护 lineCount
```

## 验证

```powershell
Get-Process msedgewebview2 | Measure-Object WorkingSet -Sum | ForEach-Object { [math]::Round($_.Sum/1MB) }
```

场景：① 启动 1 分钟 ② 失焦挂后台 10 分钟 ③ 持续看日志 30 分钟（要求不涨），
改动前后各测一轮对比。

## 相关

- [ADR-002](ADR-002-日志推送用Channel.md)（日志推送用 Channel）—— tail 推送机制不变，
  仅前端缓冲加上限
