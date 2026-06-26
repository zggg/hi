# M3：SQLite 会话持久化 + 跨进程共享

This ExecPlan is a living document.

## Purpose / Big Picture

TUI 与 `hi chat` 重启后保留对话历史；`hi tui` 与 `hi gateway` 可共享 `~/.hi/data/sessions.db`（WAL，跨进程）。

**进程内并发**（M12 ✅）：`SessionStore` 使用 **1 条写连接 + 只读连接池**（`[storage].read_pool_size`，默认 4）。旁路读（`hi session list/show`、knot 列表等）走读池；agent turn（load context、append）走写连接。详见 [m12-sqlite-concurrent-access.md](m12-sqlite-concurrent-access.md)。

## Progress

- [x] (2026-05-22) ExecPlan 创建
- [x] (2026-05-22) `SessionStore` rusqlite + schema + WAL
- [x] (2026-05-22) `AgentLoop::with_persistence` 加载/追加消息
- [x] (2026-05-22) `runtime` 默认打开 `sessions.db`
- [x] (2026-05-22) 单元测试（store + agent roundtrip）
- [x] (2026-06) M12：读写分离 + 读连接池（`ReadPool`、`open_with_pool`）

## Validation

```sh
cargo test -p hi-core -- store
cargo test -p hi-core -- agent
cargo run -p hi -- chat 记住我叫 Alice
# 重启
cargo run -p hi -- chat 我叫什么
# 应能引用上一轮（取决于模型与上下文）
```

## Key Files

```
core/src/store/mod.rs       # SessionStore（write + read_pool）
core/src/store/read_pool.rs # 只读连接池
core/src/store/schema.rs    # WAL + SCHEMA_VERSION
core/src/config/storage.rs  # [storage].read_pool_size
app/src/services.rs         # open_with_pool(config.storage…)
```

## 相关里程碑

| 里程碑 | 关系 |
|--------|------|
| M7 | append-only transcript、压缩、`mark_out_of_context` |
| M12 | 进程内读并发；M3 的 WAL 负责跨进程 |
