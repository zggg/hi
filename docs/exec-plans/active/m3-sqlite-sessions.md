# M3：SQLite 会话持久化 + 跨进程共享

This ExecPlan is a living document.

## Purpose / Big Picture

TUI 与 `hi chat` 重启后保留对话历史；`hi tui` 与 `hi gateway`（后续）可共享 `~/.hi/data/sessions.db`（WAL）。

## Progress

- [x] (2026-05-22) ExecPlan 创建
- [x] (2026-05-22) `SessionStore` rusqlite + schema + WAL
- [x] (2026-05-22) `AgentLoop::with_persistence` 加载/追加消息
- [x] (2026-05-22) `runtime` 默认打开 `sessions.db`
- [x] (2026-05-22) 单元测试（store + agent roundtrip）
- [ ] 本地 `cargo test` 验证

## Validation

```sh
cargo test -p hi-core -- store
cargo test -p hi-core -- agent
cargo run -p hi -- chat 记住我叫 Alice
# 重启
cargo run -p hi -- chat 我叫什么
# 应能引用上一轮（取决于模型与上下文）
```
