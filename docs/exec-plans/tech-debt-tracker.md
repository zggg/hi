# 技术债追踪


| 项              | 优先级    | 说明                                | 目标里程碑 |
| -------------- | ------ | --------------------------------- | ----- |
| AgentLoop 占位   | ~~P0~~ | M1 已实现纯对话 `run_turn`              | M1 ✅  |
| hi-ai 未接入 app  | ~~P1~~ | `ProviderBridge` + `hi chat`      | M1 ✅  |
| TUI/Gateway 占位 | ~~P1~~ | 消息渠道 Gateway M4 已实现             | M4 ✅  |
| 无 SQLite 实现    | ~~P1~~ | M3 `SessionStore` + 持久化 AgentLoop | M3 ✅  |
| 上下文压缩         | ~~P2~~ | M5 `maybe_compress` + `[context]` | M5 ✅  |
| 多 Provider      | ~~P2~~ | anthropic / ollama                | M5 ✅  |
| config 向导      | ~~P2~~ | `hi setup`                        | M6 ✅  |
| **记忆体系（M7）** | ~~P1~~ | 结绳 + append-only 会话 + memory 工具 | M7 ✅  |
| M4 企微联调        | P1     | 需真实 bot_id + secret             | —     |
| 无 git 仓库       | P2     | 用户未初始化 git                        | —     |
| OpenSpec 未接入   | P3     | harness-init 跳过外部依赖               | 按需    |

## M7+ 可选增强

| 项 | 优先级 | 说明 |
|----|--------|------|
| `hi session uncompress` | P3 | 将压缩段恢复为 `in_context=1`；详细设计 §17 首版不做 |
| 向量 knot 检索 | P3 | embedding / `vector_search_enabled` |
| wecom→local owner | P3 | `channel_identities` 映射企微 userid 到 `local` |

更新规则：完成项移到 exec-plans/completed/ 或删除行；可选 backlog 保留在上表。
