# M1：OpenAI 兼容 Provider + 最小 AgentLoop（纯对话）

This ExecPlan is a living document.

## Purpose / Big Picture

M1 交付一条可运行的 **纯对话** 主路径：用户在终端输入消息，经 `AgentLoop` 调用 LLM，得到 assistant 回复。无工具、无 SQLite、无 TUI/企微。

验收：`DEEPSEEK_API_KEY=... cargo run -p hi -- chat` 能进行多轮对话并看到 assistant 输出。

## Progress

- (2026-05-22) ExecPlan 创建
- (2026-05-22) `hi-core`：`LlmClient` trait + `AgentLoop::run_turn`
- (2026-05-22) `hi-ai`：`OpenAiCompatProvider`
- (2026-05-22) `hi (app)`：`ProviderBridge` + `hi chat` 子命令
- (2026-05-22) `hi-core` 单元测试（MockClient）
- 本地 `cargo test/clippy`（当前环境无 cargo，需在开发者机器验证）
- (2026-05-22) 更新 `tech-debt-tracker.md`、`ARCHITECTURE.md`、`AGENTS.md`

## Surprises & Discoveries

（实施中填写）

## Decision Log

- **Decision**：`hi-core` 定义 `LlmClient` trait，`hi-ai` 不依赖 `hi-core`；`app` 层 `ProviderBridge` 做类型转换。
**Rationale**：符合 `docs/architecture/LAYERS.md` 与 core-beliefs #3。
**Date**：2026-05-22
- **Decision**：M1 验收用 `hi chat`（stdin REPL），不提前做 ratatui TUI（属 M2）。
**Rationale**：最小可观测入口，避免 M2 范围渗入 M1。
**Date**：2026-05-22
- **Decision**：M1 会话历史仅驻内存，`SessionStore` 仍占位（M3）。
**Rationale**：先跑通 LLM 往返，持久化下一里程碑。
**Date**：2026-05-22

## Outcomes & Retrospective

（完成后填写）

## Context and Orientation

- **当前状态**：M0 完成。`AgentLoop` 为空结构体；`AiProvider` trait 在 `hi-ai` 已定义但未实现；`app` 未依赖 `hi-ai`。
- **仓库根**：`/path/to/hi`
- **5 crate**：`app/`→`hi`，`core/`→`hi-core`，`ai/`→`hi-ai`，`tui/`，`gateway/`

## Plan of Work

1. `core/src/llm.rs` — `ChatMessage`、`Role`、`LlmClient`、`LlmRequest`、`LlmResponse`
2. `core/src/agent.rs` — `AgentLoop<C: LlmClient>`，`run_turn` 追加 user/assistant，发出 `AgentEvent`
3. `ai/src/openai_compat.rs` — HTTP 调用 OpenAI 兼容 `/v1/chat/completions`
4. `app/src/bridge.rs` — `ProviderBridge` 实现 `LlmClient`
5. `app/src/main.rs` — 依赖 `hi-ai`，新增 `chat` 子命令，从 `Config` + 环境变量组装 Provider
6. `core` / `ai` 单元测试

## Concrete Steps

```sh
cd /path/to/hi
cargo build
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo test -p architecture-tests
./scripts/check-consistency.sh

# 需有效 API Key（示例 DeepSeek）
export DEEPSEEK_API_KEY=sk-...
cargo run -p hi -- chat
# 输入 hello，应看到 assistant 回复
```

## Validation and Acceptance


| 检查项                                       | 预期                                   |
| ----------------------------------------- | ------------------------------------ |
| `cargo test --workspace`                  | 全部通过                                 |
| `cargo clippy --workspace -- -D warnings` | 无 warning                            |
| architecture-tests                        | 无新违规                                 |
| `hi chat`                                 | 多轮 stdin 对话，每轮 emit 后打印 assistant 文本 |
| `hi chat 你好`                         | 单轮后退出                                |


## Idempotence and Recovery

- 步骤可重复执行；无数据库迁移。
- 若 API 失败：检查 `api_key_env` 环境变量与 `base_url`。

## Artifacts and Notes

```
> hi chat
you> 写一个 hello world rust
assistant> ```rust fn main() { println!("Hello, world!"); } ```
```

## Interfaces and Dependencies

```rust
// hi-core
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse>;
}

impl<C: LlmClient> AgentLoop<C> {
    pub async fn run_turn(&mut self, user_message: &str) -> Result<Vec<AgentEvent>>;
}

// hi-ai
pub struct OpenAiCompatProvider { ... }
impl AiProvider for OpenAiCompatProvider { ... }

// hi (app)
pub struct ProviderBridge(Arc<dyn AiProvider>);
impl LlmClient for ProviderBridge { ... }
```

新增 workspace 依赖：`reqwest`（`hi-ai`）。