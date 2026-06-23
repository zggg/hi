# M7：记忆体系建设（结绳记事 + 会话永久保留）

This ExecPlan is a living document.

> **状态**：✅ **已完成**（2026-06-23）。Phase A–E 核心全部落地；可选增强见 [M7+ backlog](#m7-backlog-可选) 与 [tech-debt-tracker.md](../tech-debt-tracker.md)。  
> **详细设计**：[结绳记事长期记忆设计](../../design/2026-06-04-knot-memory-design.md)

## Purpose

1. **结绳记事（Knot Memory）**：个人助手跨会话长期记忆（偏好、事实、待办等）。  
2. **会话永久保留（竹简）**：`messages` append-only，压缩只改 `in_context`，**禁止 DELETE**（`hi session purge` 除外）。

本地入口 `tui:main` / `chat:main` 通过 `owner_id = local` 共享 knots；各渠道 **transcript 仍按 session 隔离**，但各自全文永久保存在库。

## 实施阶段

- [x] **Phase A**：`in_context` + `mark_out_of_context` + 废止 compress 路径 `replace_messages` + session CLI
- [x] **Phase B**：knots schema + `hi memory list/add/forget/reinforce/extract`
- [x] **Phase C**：knot 注入 + 跨 session（`memory/inject` + Agent system prompt）
- [x] **Phase D**：knot 抽取 + 压缩联动（`extract.rs` / `merge.rs`）
- [x] **Phase E**：`memory_search` / `memory_write` 工具 + 基线注入（`inject_baseline_only`）

## M7+ backlog（可选）

以下项在详细设计 §17 中明确为 **M7 首版不做** 或 **M7+ 可选**，不阻塞 M7 关闭：

| 项 | 说明 |
|----|------|
| `hi session uncompress` | 将压缩段恢复为 `in_context=1` |
| 向量检索 | `vector_search_enabled` / embedding |
| wecom→local | `channel_identities` 将企微 userid 映射到 `local` owner |

## Non-goals（M7 首版，已遵守）

- 不做 Memory SaaS
- 不自动删除旧 session / 旧 message
- 首版不上向量库

## Progress

- [x] 结绳详细设计
- [x] 会话永久保留写入设计（§4、§8）
- [x] **Phase A**：`in_context`、append-only、`mark_out_of_context`、`hi session` CLI、架构测试
- [x] **Phase B**：knots schema、`MemoryConfig`、`hi memory` CLI
- [x] **Phase C**：knot 注入 + 跨 session（owner=local）
- [x] **Phase D**：knot 抽取 + 压缩联动
- [x] **Phase E**：`memory_search` / `memory_write` + 基线注入

## Validation

见 [详细设计 §14](../../design/2026-06-04-knot-memory-design.md#14-验证计划)。

```sh
cargo test -p hi-core -- store memory
cargo test -p architecture-tests
./scripts/check-consistency.sh
```
