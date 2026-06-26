# M10 Gateway Common 抽象

> 里程碑编号：**M10**（已完成）。后续 HTTP 接口计划为 [M11](m11-http-gateway-endpoint.md)。

> 状态：**已完成**（P1–P4）  
> 目标：三渠道共享 turn 编排，新渠道只实现协议层。

## 循环目标（按阶段验收）

| 阶段 | 交付物 | 验收标准 |
|------|--------|----------|
| **P1** | `common/approval.rs` `turn.rs` `dead_letter.rs` | 三渠道删除重复 ApprovalBus / process_turn 重试 / dead_letter |
| **P2** | `common/messenger.rs` + `ChannelApproval<M>` | 三渠道删除重复 ApprovalHandler 实现 |
| **P3** | `common/reply.rs` `hooks.rs` `user_error.rs` | 分片投递、微信 typing/welcome、空回复规范化统一 |
| **P4** | `ws_lifecycle.rs` `dedup.rs` `config_warn.rs` `concurrency.rs` | 飞书/企微重连循环、去重、dm_policy 警告、semaphore spawn |

## 模块地图

```
gateway/src/common/
├── approval.rs      ApprovalBus + ChannelApproval<M>
├── turn.rs          TurnContext, run_agent_turn, process_turn_with_retry
├── dead_letter.rs   record_dead_letter(channel, ...)
├── messenger.rs     ChannelMessenger trait
├── reply.rs         ReplySink trait
├── hooks.rs         TurnHooks + NoopTurnHooks
├── user_error.rs    normalize_reply_parts, user_visible_error
├── concurrency.rs   spawn_bounded_turn
├── ws_lifecycle.rs  reconnect_loop (Feishu / WeCom)
├── dedup.rs         TimedDedup, IdDedup
└── config_warn.rs   warn_dm_policy, warn_feishu_mention
```

## 新渠道接入清单

1. 实现 **Transport**（连接、收包、心跳）
2. 解析为 `user_key` + `text` + reply target
3. 实现 `ChannelMessenger` + `ReplySink`
4. 可选 `TurnHooks`（thinking / busy / typing）
5. 调用 `process_turn_with_retry` + `ApprovalBus.try_resolve`

## 刻意不抽象

- 飞书 protobuf 分片 / ACK / tenant token / 群 @
- 企微 stream req_id / aibot_* 命令
- 微信 long-poll / context_token / session pause

## 验证

```sh
cargo test -p hi-gateway
cargo clippy -p hi-gateway -- -D warnings
```
