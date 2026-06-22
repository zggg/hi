# Core Beliefs

以下决策**不可违背**。变更需写 ADR 并更新本文档。

## 1. 平台无关 core

TUI 与 Gateway 只做 I/O，不含 Agent 业务逻辑。所有 Agent 行为在 `hi-core`。

## 2. 会话按渠道隔离

每个入口独立会话，**不**跨渠道共享历史：

- TUI → `tui:main`
- `hi chat` → `chat:main`
- 远程渠道用户（如 wecom）→ `wecom:{userid}`

渠道是传输层；会话边界由 `session_id` 决定。

## 3. hi-ai 由 app 组装

`hi-core` 不直接依赖 `hi-ai`。Provider 在 `hi (app)` 层注入，保持 AI 层与 core 解耦。

## 4. 最小工具集

MVP 核心四工具：read、write、edit、bash。M7 扩展 `memory_search` / `memory_write`（`[memory]` 配置控制）。不提前引入 MCP/Skills。

## 5. 机械架构强制

分层规则由 boundary test 强制执行，不靠 code review 口头约定。

## 6. 消息渠道 Gateway

Gateway 只做 I/O 适配，与 core 解耦。已接入：**企业微信**（WebSocket）、**飞书**（WebSocket）、**个人微信 iLink**（HTTP 长轮询，实验性）。配置写在 `~/.hi/hi.toml` 的 `[channels.*]` 段（旧版 `[wecom]` 仍可加载）。
