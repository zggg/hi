# M7：记忆体系建设（结绳记事 + 会话永久保留）

This ExecPlan is a living document.

> **状态**：Phase A–E（部分）已落地；Phase E 其余待排期。  
> **详细设计**：[结绳记事长期记忆设计](../../design/2026-06-04-knot-memory-design.md)

## Purpose

1. **结绳记事（Knot Memory）**：个人助手跨会话长期记忆（偏好、事实、待办等）。  
2. **会话永久保留（竹简）**：`messages` append-only，压缩只改 `in_context`，**禁止 DELETE**（`hi session purge` 除外）。

本地入口 `tui:main` / `chat:main` 通过 `owner_id = local` 共享 knots；各渠道 **transcript 仍按 session 隔离**，但各自全文永久保存在库。

## 当前问题（M3/M5）

| 问题 | 说明 |
|------|------|
| **有损压缩即删除** | `maybe_compress` → `replace_messages` → `DELETE FROM messages` |
| 无上下文/全文分离 | Agent 与 DB 共用同一视图，压缩即丢原文 |
| 无长期记忆层 | 仅有 transcript + 可选摘要 |
| 摘要不可控 | 关键决策可能漏掉 |

## 方案摘要

| 组件 | 职责 |
|------|------|
| **messages.in_context** | `1` = Agent 可见；`0` = 已压缩出上下文，**行仍在库** |
| **session_compressions** | 压缩事件：message id 范围 + 可选 summary |
| **knots** | 长期记忆（5 类 atomic） |
| **忘川 clarity** | knot 可衰减；**会话不可自动 TTL** |

## 实施阶段

- [x] **Phase A**：`in_context` + `mark_out_of_context` + 废止 compress 路径 `replace_messages` + session CLI
- [x] **Phase B**：knots schema + `hi memory list/add/forget/reinforce`
- [x] **Phase C**：knot 注入 + 跨 session（`memory/inject` + Agent system prompt）
- [x] **Phase D**：knot 抽取 + 压缩联动（`extract.rs` / `merge.rs` / `hi memory extract`）
- [x] **Phase E（部分）**：`memory_search` 工具 + 基线注入重构
- [ ] Phase E 其余：uncompress、向量、wecom→local

## Non-goals（M7 初期）

- 不做 Memory SaaS
- 不自动删除旧 session / 旧 message
- 首版不上向量库

## Progress

- [x] 结绳详细设计
- [x] 会话永久保留写入设计（§4、§8）
- [x] **Phase A**：`in_context`、append-only、`mark_out_of_context`、`hi session` CLI、架构测试
- [x] **Phase C**：knot 注入 + 跨 session
- [x] **Phase D**：knot 抽取 + 压缩联动
- [x] **Phase E（部分）**：`memory_search` + 基线注入
- [ ] Phase E 其余

## Validation

见 [详细设计 §14](../../design/2026-06-04-knot-memory-design.md#14-验证计划)。

```sh
cargo test -p hi-core -- store
cargo test -p architecture-tests
./scripts/check-consistency.sh
```
