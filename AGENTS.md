# hi — Agent 导航地图

> 极致轻量化的个人 AI 助手：本地 TUI + 消息渠道 Gateway，共用同一 Agent 核心（Rust Cargo Workspace）。

## 技术栈

| Layer     | Tech                          |
|-----------|-------------------------------|
| Language  | Rust 2021                     |
| Async     | tokio                         |
| CLI       | clap                          |
| TUI       | ratatui + crossterm           |
| HTTP      | reqwest（LLM API）；Gateway 走 WebSocket |
| Storage   | rusqlite WAL                  |
| Config    | TOML + serde                  |
| Logging   | tracing                       |

## 架构分层

依赖只能**向下**流动。5 个 crate，按职责分层：

```
entry      hi (app/)           CLI 入口与组装
adapters   hi-tui, hi-gateway  I/O 层（终端 / 消息渠道）
foundation hi-core, hi-ai       平台无关核心 + LLM Provider
```

详细规则见 [docs/architecture/LAYERS.md](docs/architecture/LAYERS.md)。

## 关键约定

- **平台无关 core**：Agent 逻辑只在 `hi-core`；TUI/Gateway 只做 I/O → [docs/golden-principles/IMPORTS.md](docs/golden-principles/IMPORTS.md)
- **共享运行时**：`HiServices`（`app/src/services.rs`）持有共享 `SessionStore`（1 写 + 读池）、LLM Provider、`SessionCoordinator`；Gateway 通过 `PersistedAgentHost` 调用，禁止每回合新建 store
- **会话按渠道隔离**：`tui:main` / `chat:main` / `wecom:{userid}` 互不共享 → [docs/design-docs/core-beliefs.md](docs/design-docs/core-beliefs.md)
- **错误带上下文**：使用 `hi_core::Error`，工具/边界测试输出含修复指引 → [docs/golden-principles/ERROR_HANDLING.md](docs/golden-principles/ERROR_HANDLING.md)
- **命名与 crate 对齐**：目录 `app/` 对应 package `hi`；其余 crate 目录名与 package 名一致 → [docs/golden-principles/NAMING.md](docs/golden-principles/NAMING.md)

## 命令

```sh
cargo build                              # 编译全部 crate
cargo test -p architecture-tests         # 架构边界测试
cargo run -p hi                          # 本地 TUI（默认，同 hi tui）
cargo run -p hi -- tui                   # 本地 TUI（显式）
cargo run -p hi -- chat                  # 终端对话（stdin）
cargo run -p hi -- chat 你好        # 单轮对话
cargo run -p hi -- gateway               # 消息渠道网关
cargo run -p hi -- gateway --check       # 渠道连接预检
cargo run -p hi -- config            # 查看配置（密钥脱敏）
cargo run -p hi -- setup                 # 基础配置：LLM + 工作目录（已存在则回车保留当前值）
cargo run -p hi -- model                 # 仅配置模型：新增/切换 model，保留工作目录与渠道
cargo run -p hi -- gateway setup         # 消息渠道配置
cargo run -p hi -- session list           # 会话列表（全文永久保留）
cargo run -p hi -- session show --session chat:main
cargo run -p hi -- session show --context # 仅 Agent 上下文
cargo run -p hi -- memory list             # 结绳长期记忆
cargo run -p hi -- memory add "偏好简体中文" --kind preference --confirmed
cargo run -p hi -- memory extract          # LLM 从会话 transcript 抽结
# Agent 可用 memory_search 工具按需查记忆（待办/决定等）；system 仅注入偏好+事实基线
cargo test --workspace                     # 全 workspace 单测
cargo clippy --workspace -- -D warnings  # Lint
./scripts/check-consistency.sh           # 边界测试 + 单测 + clippy + 文档一致性
```

## 文档地图

```
ARCHITECTURE.md                       顶层领域地图（根目录）
docs/
├── architecture/LAYERS.md            分层规则、依赖图、修复指引
├── golden-principles/                典范模式（DO/DON'T）
├── SECURITY.md                       认证、密钥、威胁模型
├── STACK.md                          Rust 技术栈约定
├── guides/                           setup、testing
├── exec-plans/                       功能实施计划
├── design-docs/                      架构决策与 core beliefs
├── design/                           原始设计文档
│   └── 2026-05-22-hi-agent-design.md
```

## 从哪里开始看

| 任务               | 先看这里                              |
|--------------------|---------------------------------------|
| 架构概览           | ARCHITECTURE.md                       |
| 分层与 crate 依赖  | docs/architecture/LAYERS.md           |
| 改 Agent 核心      | core/src/                             |
| 改 LLM Provider    | ai/src/provider.rs                    |
| 改 CLI 入口 / 运行时组装 | app/src/main.rs · app/src/services.rs |
| 改 TUI             | tui/src/                              |
| 改消息渠道 Gateway | gateway/src/                          |
| 企业微信渠道联调   | docs/guides/wecom-gateway-integration.md |
| 飞书渠道联调       | docs/guides/feishu-gateway-integration.md |
| 个人微信 iLink 联调 | docs/guides/weixin-ilink-integration.md |
| MVP 里程碑         | docs/design/2026-05-22-hi-agent-design.md |
| M3 会话持久化      | docs/exec-plans/active/m3-sqlite-sessions.md |
| M4 消息渠道 Gateway | docs/exec-plans/active/m4-wecom-gateway.md |
| M5 压缩 + Provider | docs/exec-plans/active/m5-context-and-providers.md |
| M6 配置向导        | docs/exec-plans/active/m6-config-wizard.md |
| M7 记忆体系（结绳记事） | docs/exec-plans/active/m7-memory-system.md · [详细设计](docs/design/2026-06-04-knot-memory-design.md) |
| M8 个人微信 iLink  | docs/exec-plans/active/m8-weixin-ilink-gateway.md · [详细设计](docs/design/2026-06-09-weixin-ilink-gateway-design.md) |
| M9 国际化 i18n     | docs/exec-plans/active/m9-i18n.md |
| M10 Gateway 公共抽象（已完成） | docs/exec-plans/active/m10-gateway-common.md |
| M11 HTTP 接口（gateway endpoint） | docs/exec-plans/active/m11-http-gateway-endpoint.md |
| M12 SQLite 并发访问（1 写 + 读池，已完成） | docs/exec-plans/active/m12-sqlite-concurrent-access.md |

## 约束（机器可读）

- MUST: 新增 crate 间依赖前查 LAYERS.md；违规时 boundary test 失败
- MUST NOT: `hi-core` / `hi-ai` 依赖 `hi-tui` / `hi-gateway` / `hi`
- MUST NOT: `hi-tui` 与 `hi-gateway` 互相依赖
- PREFER: 最小 diff；匹配现有模块风格；不提前实现未到达的里程碑
- VERIFY: `cargo test --workspace && ./scripts/check-consistency.sh`
