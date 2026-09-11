# ADR-002: 日志/构建输出用 ipc::Channel 推送

**状态**：部分被取代（见 ADR-006）
**日期**：阶段 0

> **后续修订**：本 ADR 中「依赖 Channel GC 自动退订」的部分已被
> [ADR-006](ADR-006-日志订阅生命周期与隔离.md) 取代 —— GC 时机不确定，会导致
> 旧订阅继续把日志写入新缓冲。现在改为显式 `unsubscribe_log(sub_id)`。
> 「用 Channel 而非 Event 系统」这一选型决策仍然有效。

## 背景

DevLauncher 有两类实时数据需要从 Rust 推送到前端：
1. **运行时日志流**（tail 日志文件，持续推送新行）
2. **构建命令输出**（stdout/stderr 实时推送）

Tauri 2 提供两种推送机制：
- **Event 系统**：`app.emit("event_name", payload)` + 前端 `listen()`
- **ipc::Channel**：command 参数传 `Channel<T>`，Rust 端 `channel.send(payload)`

## 决策

**使用 `tauri::ipc::Channel<T>`，不用 Event 系统。**

## 理由

1. **生命周期自动管理**：Channel 对象在前端 GC 时，Rust 端 `channel.send()` 会返回错误，tail task 自动退出。Event 系统需要手动 `unlisten()`，且 Rust 端不知道前端是否还在监听（容易产生泄漏的 tokio task）。

2. **一对一绑定**：每个项目的日志订阅是一个独立 Channel，天然隔离。Event 系统需要用 event_name 命名空间（`"log-1"`, `"log-2"`）手动管理，容易冲突。

3. **类型安全**：Channel 是泛型 `Channel<LogChunk>`，Rust 端编译期保证类型；Event 是动态序列化。

4. **Tauri 官方推荐**：Tauri 2 文档明确推荐用 Channel 处理「command 持续返回数据」场景。

## 代价

- Channel 必须作为 command 参数传入（不能在任意 Rust 代码里 emit），所以订阅必须在 invoke 时建立
- 前端需要持有 Channel 引用防止 GC（在 store 中保存）

## 实现

```rust
#[tauri::command]
async fn subscribe_log(
    project_id: i64,
    on_event: Channel<LogChunk>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    // 启动 tokio tail task，轮询日志文件
    state.tail_registry().spawn(project_id, on_event).await
}
```

```typescript
// 前端
const channel = new Channel<LogChunk>()
channel.onmessage = (chunk) => { logStore.append(chunk.text) }
await invoke('subscribe_log', { projectId: 1, onEvent: channel })
```

## 相关

- ADR-001（进程管理）—— stdout/stderr 落盘后，tail task 读文件推 Channel
