# hi 架构概览

hi 是 **极致轻量化的个人 AI 助手**（Cargo Workspace，5 个 crate），实现「本地 TUI + 远程渠道 Gateway 共用 Agent 核心」。

## 领域地图

```
┌─────────────────────────────────────────────────────────┐
│  hi (app/) — CLI 入口：tui / gateway / config           │
└──────────────┬──────────────────────────┬───────────────┘
               │                          │
               ▼                          ▼
┌──────────────────────────┐   ┌──────────────────────────┐
│  hi-tui                  │   │  hi-gateway              │
│  终端交互界面             │   │  远程消息渠道适配器       │
└──────────────┬───────────┘   └──────────────┬───────────┘
               │                              │
               └──────────────┬───────────────┘
                              ▼
               ┌──────────────────────────────┐
               │  hi-core                     │
               │  AgentLoop · tools · store   │
               │  config · session · events   │
               └──────────────┬───────────────┘
                              │
               ┌──────────────┴───────────────┐
               │  hi-ai（独立叶子 crate）      │
               │  AiProvider trait            │
               └──────────────────────────────┘
```

`hi-ai` 由 `hi (app)` 组装进 Agent 运行时，**不**被 `hi-core` 直接依赖（保持 core 平台无关）。

## 当前状态

- **M0 完成**：Workspace 骨架、占位 CLI、类型与 trait 定义
- **M1 完成**：`OpenAiCompatProvider`、`AgentLoop` 纯对话、`hi chat`
- **M2 完成**：四工具、bash 审批、`~/.hi/hi.toml`、`hi tui`（ratatui）
- **M3 完成**：SQLite WAL 会话（`~/.hi/data/sessions.db`）、`AgentLoop::with_persistence`
- **M4 完成**：消息渠道 Gateway（企业微信智能机器人 WebSocket；会话 `wecom:{userid}`）
- **M5 完成**：上下文压缩、Anthropic / Ollama Provider
- **M6 完成**：`hi setup` / `hi gateway setup` 交互向导、统一 `~/.hi/hi.toml`
- **M7 进行中**：结绳长期记忆、`memory_search` / `memory_write`、会话 append-only（见 [m7-memory-system.md](docs/exec-plans/active/m7-memory-system.md)）
- **M8 MVP 完成**：个人微信 iLink Gateway（实验性；真实环境联调见 [m8-weixin-ilink-gateway.md](docs/exec-plans/active/m8-weixin-ilink-gateway.md)）

## 进一步阅读

- 分层规则与修复指引 → [docs/architecture/LAYERS.md](docs/architecture/LAYERS.md)
- 设计决策与 MVP 范围 → [docs/design-docs/core-beliefs.md](docs/design-docs/core-beliefs.md)
- 完整设计文档 → [docs/design/2026-05-22-hi-agent-design.md](docs/design/2026-05-22-hi-agent-design.md)
