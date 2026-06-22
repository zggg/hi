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
| **记忆体系**       | **P1** | Phase A 完成：append-only + in_context；结绳 Phase B 待做 | **M7 进行中** |
| M4 企微联调        | P1     | 需真实 bot_id + secret             | —     |
| 无 git 仓库       | P2     | 用户未初始化 git                        | —     |
| OpenSpec 未接入   | P3     | harness-init 跳过外部依赖               | 按需    |


更新规则：完成项移到 exec-plans/completed/ 或删除行。
