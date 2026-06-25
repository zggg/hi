# hi

[![CI](https://github.com/zggg/hi/actions/workflows/ci.yml/badge.svg)](https://github.com/zggg/hi/actions/workflows/ci.yml)

English: [README.md](README.md)

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

## 命令一览

| 命令 | 说明 |
|------|------|
| `hi` / `hi tui` | 启动本地 TUI（`-s <会话>` 指定会话，`-v` 详细模式） |
| `hi chat [消息]` | 终端对话：带参数=单轮，无参=stdin REPL |
| `hi setup` | 配置向导：LLM + 工作目录（已配置则回车保留当前值） |
| `hi model` | 仅配置模型：新增 / 切换 model，保留工作目录与渠道 |
| `hi config` | 查看当前配置（密钥脱敏） |
| `hi gateway` | 消息渠道网关；`--check` 连接预检 |
| `hi gateway <动作>` | `setup` / `start` / `stop` / `restart` / `status` / `reload` / `run` |
| `hi session <子命令>` | 会话：`list` / `show` / `export` / `compressions` / `compression-show` / `purge` |
| `hi memory <子命令>` | 结绳记忆：`list` / `show` / `add` / `forget` / `reinforce` / `extract` |

> 任意命令加 `--help` 查看完整参数，例如 `hi gateway --help`、`hi session show --help`。

## 可用工具

Agent 内置 4 个核心工具，记忆开启后再追加 2 个（受 `[memory]` 配置控制）：

| 工具 | 作用 | 审批 |
|------|------|------|
| `read` | 读取文件，相对路径基于工作目录 | 工作区外路径首次需审批（按目录树记忆） |
| `write` | 写入文件（整体覆盖） | 工作区外路径首次需审批 |
| `edit` | 替换文件中首个匹配的 `old_string` | 工作区外路径首次需审批 |
| `bash` | 在工作目录执行 shell 命令 | 危险命令与工作区外写入需一次性审批；hardline 命令不可放行 |
| `memory_search` *(可选)* | 按关键词检索结绳长期记忆（待办 / 决定 / 流程 / 事实） | — |
| `memory_write` *(可选)* | 主动保存值得跨会话记住的持久记忆，自动去重 | — |

> 审批结果写入 `tools.approvals`；`mode = "off"` 可关闭审批。`memory_search` / `memory_write` 分别由 `memory_search_enabled` / `memory_write_tool` 控制。

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
