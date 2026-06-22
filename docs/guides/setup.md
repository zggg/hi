# 开发环境 Setup

> 终端用户从零安装请看 [install.md](install.md)。完整引导流程见 [onboarding-flow.md](onboarding-flow.md)。

## 要求

- [Rust](https://rustup.rs)（2021 edition）
- 大模型 API（OpenAI 兼容、Anthropic、Codex 或本机 Ollama）
- 消息渠道（可选）：**企业微信**、**飞书**、**个人微信 iLink**（实验性）

## 克隆与编译

```sh
cd /path/to/hi
cargo build
```

## 配置

**推荐**：交互式向导（密钥写入 `~/.hi/hi.toml`，无需环境变量）

```sh
cargo run -p hi -- setup           # LLM + workspace（[context] 默认启用）
cargo run -p hi -- gateway setup   # 消息渠道（可选，支持 wecom / feishu / weixin）
cargo run -p hi -- config          # 查看配置（密钥脱敏）
```

或手动复制示例：

```sh
mkdir -p ~/.hi
cp hi.example.toml ~/.hi/hi.toml
# 编辑 hi.toml：填写 [ai.providers.<name>] 等
```

| 字段 | 说明 |
|------|------|
| `workspace` | Gateway 工作区（`hi gateway`）；`hi`/`tui`/`chat` 用**当前目录** |
| `[ai].default` | 激活的 LLM 实例名（如 `openai-compat`） |
| `[ai.providers.<name>]` | 各 LLM 实例：`provider`、`model`、`base_url`、`api_key` |
| `[context]` | 上下文压缩（默认启用） |
| `[memory]` | 结绳长期记忆（默认启用，含回合后抽取） |
| `[channels.wecom]` 等 | 消息渠道凭证（仅 gateway） |

配置文件路径：`~/.hi/hi.toml`（可用环境变量 `HI_TOML` 覆盖）。

> 旧版扁平段 `[wecom]` 仍可加载，但新配置与向导均写入 `[channels.*]`。

### 会话隔离（各入口独立）

| 入口 | session_id | 说明 |
|------|------------|------|
| `hi tui` | `tui:main` | 本地终端 |
| `hi chat` | `chat:main` | 命令行 REPL |
| 企微用户 A | `wecom:A` | 每个远程用户独立会话 |
| 飞书用户 | `feishu:{open_id}` | 同上 |
| 个人微信 iLink | `weixin:main` | 仅本人私聊 |

数据库：`~/.hi/data/sessions.db`（WAL）。schema 升级不兼容时需备份后删除重建。

### 消息渠道（可选，无需公网穿透）

| 渠道 | 协议 | 联调文档 |
|------|------|----------|
| 企业微信 | WebSocket 长连接 | [wecom-gateway-integration.md](wecom-gateway-integration.md) |
| 飞书 | WebSocket 长连接 | [feishu-gateway-integration.md](feishu-gateway-integration.md) |
| 个人微信 iLink | HTTP 长轮询（实验性） | [weixin-ilink-integration.md](weixin-ilink-integration.md) |

```sh
hi gateway setup       # 交互选渠道并填凭证
hi gateway --check     # 预检，成功后退出
hi gateway             # 常驻运行
```

访问控制（企微/飞书，**默认 `allowlist`**，须配置 `allow_from`；联调可设 `open`）：

```toml
[channels.wecom]
bot_id = "your-bot-id"
secret = "your-secret"
dm_policy = "allowlist"
allow_from = ["YourWeComUserId"]
```

## 运行

```sh
cargo run -p hi              # TUI（同 hi tui）
cargo run -p hi -- chat
cargo run -p hi -- gateway
```

## 验证

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
./scripts/check-consistency.sh
```
