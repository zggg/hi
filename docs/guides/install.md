# 安装 hi（从零开始）

hi 是极致轻量化的个人 AI 助手。按顺序做一遍即可，全程在终端操作。

> 完整流程图与向导逐步说明见 [onboarding-flow.md](onboarding-flow.md)（含待审核项）。  
> 开发者 clone 后改代码请看 [setup.md](setup.md)。

---

## 第 0 步：你需要什么

| 依赖 | 必须？ | 说明 |
|------|--------|------|
| macOS / Linux | 是 | 当前主要开发与测试平台 |
| Rust（含 cargo） | 是 | [rustup.rs](https://rustup.rs) 安装 |
| 大模型 API Key | 是 | DeepSeek / OpenAI 等，或本地 Ollama |
| 消息渠道 | 否 | 仅 `hi gateway` 需要（企业微信 / 飞书 / 个人微信 iLink） |

---

## 第 1 步：安装 Rust（如果还没有 cargo）

```bash
cargo --version
```

若提示 `command not found`：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# 安装过程中选 1) default

source "$HOME/.cargo/env"
cargo --version
```

下载慢时，按你的网络环境设置代理（示例，端口请改成你自己的）：

```bash
export https_proxy=http://127.0.0.1:PORT
export http_proxy=http://127.0.0.1:PORT
```

**以后新开终端**若找不到 `cargo`，先 `source "$HOME/.cargo/env"`，或写入 `~/.zshrc`。

---

## 第 2 步：进入 hi 项目目录

```bash
cd /path/to/hi    # 换成你的 clone 路径
ls Cargo.toml app/ core/
```

---

## 第 3 步：编译 hi

```bash
source "$HOME/.cargo/env"
cd /path/to/hi
cargo build -p hi
```

第一次会下载依赖，可能需要几分钟。成功时最后一行类似：

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in ...
```

验证（**不安装也能用**）：

```bash
cargo run -p hi -- --help
```

应看到子命令：`tui` `chat` `gateway` `config` `session` `memory` 等。

---

## 第 4 步：安装到 PATH（推荐）

```bash
cargo install --path app
which hi          # 一般为 ~/.cargo/bin/hi
hi --help
```

| 方式 | 命令示例 | 何时用 |
|------|----------|--------|
| **已安装** | `hi chat` | 日常推荐 |
| **未安装** | `cargo run -p hi -- chat` | 刚 clone、还在改代码 |

下文统一写 `hi ...`；未执行第 4 步时，把 `hi` 换成 `cargo run -p hi --`。

---

## 第 5 步：配置大模型（必做）

```bash
hi setup
```

交互向导（cliclack 菜单，**箭头选择**，不是输入数字）。写入 **`~/.hi/hi.toml`**。

**顺序**：先配大模型，再可选配消息渠道；跳过渠道时稍后可 `hi gateway setup` 补配（支持企微 / 飞书 / 个人微信 iLink）。

| 步骤 | 内容 |
|------|------|
| 大模型 | Provider → model → API Key（Ollama / Codex 可跳过 Key） |
| 消息渠道（可选） | 默认跳过；选「现在配置」后：渠道 → workspace → 凭证 → 白名单 |
| Gateway workspace | 默认 `~/.hi/workspace`，**仅 Gateway** 使用；本地 tui/chat 用 cwd |

向导**还会说明**：`[memory]` 长期记忆默认开启（可用 `hi memory list` 查看）。  
主向导里渠道步骤可跳过；欢迎语、飞书群聊 @ 策略等进阶项请用 `hi gateway setup`。

### 工作目录：Gateway vs 本地 CLI（易混淆）

| 入口 | 工具 read/bash 的工作目录 |
|------|---------------------------|
| `hi` / `hi tui` / `hi chat` | 你**执行命令时**的当前目录（`cwd`） |
| `hi gateway`（远程渠道） | `hi.toml` 里的 **`workspace`**（默认 `~/.hi/workspace`） |

查看配置：

```bash
hi config
```

---

## 第 6 步：第一次对话（安装成功标志）

```bash
hi chat 你好，回复 OK
```

有正常文字回复 → **安装 + 配置 OK**。

---

## 第 7 步：消息渠道（可选）

需先完成第 5 步。

| 渠道 | 联调文档 |
|------|----------|
| 企业微信 | [wecom-gateway-integration.md](wecom-gateway-integration.md) |
| 飞书 | [feishu-gateway-integration.md](feishu-gateway-integration.md) |
| 个人微信 iLink | [weixin-ilink-integration.md](weixin-ilink-integration.md) |

```bash
hi gateway setup      # 交互选渠道并填凭证
hi gateway --check    # 连接预检，成功后退出
hi gateway            # 常驻运行
```

---

## 常见问题

### `cargo: command not found`

```bash
source "$HOME/.cargo/env"
```

### 下载依赖很慢 / 卡住

设置代理后重试 `cargo build`（见第 1 步）。

### `hi: command not found` 但 cargo 有

```bash
cargo install --path app
# 或临时
cargo run -p hi -- chat
```

### `missing LLM api_key in ~/.hi/hi.toml`

在 `[ai.providers.<name>]` 填写 API Key，或重新运行：

```bash
hi setup
```

### `invalid workspace` / Gateway 工作区不存在

向导一般会创建目录；若手改 `hi.toml`，确保 `workspace` 指向的目录存在：

```bash
mkdir -p ~/.hi/workspace
```

本地 `hi tui` / `hi chat` 请在**你要操作的项目目录**下执行，与 `workspace` 无关。

### 对话变慢或超时、上下文过大

失败回合会自动回滚；仍异常时可对话内 `/reset` 清空 Agent 可见上下文，或见 [commands-inventory.md](commands-inventory.md)。

---

## 装好之后

| 想做什么 | 命令 |
|----------|------|
| 命令行聊天 | `hi chat` |
| 终端 UI | `hi` 或 `hi tui` |
| 看会话 / 清空上下文 | `hi session list` · 对话内 `/reset` |
| 全功能自测 | [local-test-checklist.md](local-test-checklist.md) |
| 命令全集 | [commands-inventory.md](commands-inventory.md) |

---

## 一条龙（已有 Rust）

```bash
source "$HOME/.cargo/env"
cd /path/to/hi
cargo install --path app

hi setup
hi chat 你好
```
