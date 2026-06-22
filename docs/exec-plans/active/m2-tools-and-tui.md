# M2：四工具 + bash 审批 + TUI 基础交互

This ExecPlan is a living document.

## Purpose / Big Picture

在 M1 纯对话基础上，交付 **ReAct 工具循环** 与 **终端 UI**：

- `read` / `write` / `edit` / `bash` 四工具（路径限制在 `working_directory`）
- 危险 `bash` 命令需用户确认（TUI 弹窗 / `hi chat` 提示 y/n）
- `~/.hi/config.toml` 加载（自部署 `base_url` / `model`）
- `hi tui` 可用（消息区 + 输入区 + 状态栏）

## Progress

- [x] (2026-05-22) ExecPlan 创建
- [x] (2026-05-22) config 加载 + 可选 API Key
- [x] (2026-05-22) 四工具 + ToolRegistry
- [x] (2026-05-22) LLM tool_calls（OpenAI compat）
- [x] (2026-05-22) AgentLoop ReAct + ApprovalHandler
- [x] (2026-05-22) hi-tui ratatui
- [x] (2026-05-22) `config.example.toml`、setup 文档
- [ ] 本地 `cargo test/clippy`（需在开发者机器验证）

## Decision Log

- **Decision**：`hi-core` 定义 `AgentSession` + `ApprovalHandler`，TUI 仅依赖 core。
  **Rationale**：遵守分层，app 组装具体 `AgentLoop<ProviderBridge>`。
  **Date**：2026-05-22

## Validation and Acceptance

```sh
# 配置自部署模型
mkdir -p ~/.hi
# 编辑 ~/.hi/config.toml（见设计文档）

cargo test --workspace
cargo run -p hi -- tui
# 输入：读取 README.md 前几行
# 触发 bash 危险命令时应出现审批

cargo run -p hi -- chat 列出当前目录
```
