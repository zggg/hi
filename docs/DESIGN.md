# 设计哲学

hi 是 **极致轻量化的个人 AI 助手**：Rust 实现，极简内核 + 多渠道接入；远程渠道按适配器扩展，当前首个实现为企业微信（wecom）。

## 核心原则

1. **平台无关 core** — Agent 逻辑与 I/O 分离
2. **会话按渠道隔离** — `tui:main` / `chat:main` / `wecom:{userid}` 互不共享
3. **少 crate、多 mod** — tools/store 在 core 内模块化
4. **成熟技术栈** — tokio、clap、serde 等
5. **机械强制架构** — 边界测试 + lint

## MVP 边界

做：4 工具、SQLite、多 Provider、TUI、消息渠道 Gateway（企微）、上下文压缩、结绳记忆

不做：Skills、MCP、子 Agent、Cron、个人微信、多租户 SaaS

## 文档关系

- 架构地图 → [ARCHITECTURE.md](../ARCHITECTURE.md)
- 分层规则 → [architecture/LAYERS.md](architecture/LAYERS.md)
- 不可违背决策 → [design-docs/core-beliefs.md](design-docs/core-beliefs.md)
- 完整设计 → [design/2026-05-22-hi-agent-design.md](design/2026-05-22-hi-agent-design.md)
