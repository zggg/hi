# hi

[![CI](https://github.com/zggg/hi/actions/workflows/ci.yml/badge.svg)](https://github.com/zggg/hi/actions/workflows/ci.yml)

English: [README.en.md](README.en.md)

**Rust AI Agent** — 轻量个人 AI 助手：**单二进制** · 本地 TUI · 可选 IM Gateway · 同一 Agent 核心

> **个人学习项目** — 用于练习 Rust、Agent 运行时、TUI 与消息渠道集成。功能持续迭代，**不承诺生产可用**；欢迎交流，请勿作为关键业务依赖。

## 为什么用 hi

- **体积极小、秒级启动** — Release 单二进制约十兆级，无额外运行时；冷启动即可 `hi chat` / `hi tui`
- **够轻** — 四工具 + 可选记忆，无 MCP / Skills 包袱，配置一个文件搞定
- **一核多端** — TUI、终端、企微 / 飞书 / 微信 iLink 共用 Agent；会话按渠道隔离
- **本地优先** — SQLite 持久化、`~/.hi/hi.toml` 统一配置，密钥不出本机
- **可审可控** — bash 与范围外文件默认审批；危险命令 hardline，不可放行
- **多模型** — OpenAI 兼容 · Anthropic · Ollama · Codex；Gateway 长连接，无需公网回调

## 快速开始

```bash
cargo install --path app
hi setup
hi chat 你好          # 或 hi / hi tui
```

### npm

```bash
npm install -g @i99/hi
hi setup
```

文档：[安装指南](docs/guides/install.md) · [架构](ARCHITECTURE.md) · [安全](docs/SECURITY.md)

## 记忆系统（结绳记事）

会话 transcript 与长期记忆**分层存储**：

| 层 | 存什么 | 特点 |
|----|--------|------|
| **竹简**（`messages`） | 每轮对话原文 | 只追加不删；压缩仅影响 LLM 上下文，库内全文可导出 |
| **结绳**（`knots`） | 提炼后的原子事实 | 跨 TUI / chat / Gateway 共享；一句一结 |

- **轻量** — SQLite + 类型 + 关键词，不依赖向量库
- **慢写快读** — 回合结束或压缩时 LLM 抽取；注入时只做 SQL
- **可遗忘** — `clarity` 衰减；支持手动 add / 强化 / 删除
- **按需检索** — `memory_search` 查待办与决定；`memory_write` 供 Agent 主动记；system 默认只注入偏好与事实基线

```bash
hi memory list
hi memory add "偏好简体中文" --kind preference --confirmed
hi memory extract
```

详细设计：[结绳记事](docs/design/2026-06-04-knot-memory-design.md)

## 语言 / Locale

- 未配置 `[locale]`：每次启动跟随系统 `LANG` / `LC_*`（`zh*` → 中文，其余 → English）
- 固定语言：在 `~/.hi/hi.toml` 写入 `[locale] lang = "zh"` 或 `"en"`
- 临时覆盖：`HI_LOCALE=zh|en`

> 消息渠道、Codex、微信 iLink 等依赖第三方服务条款，使用前请自行确认合规性。

## License

[MIT](LICENSE)
