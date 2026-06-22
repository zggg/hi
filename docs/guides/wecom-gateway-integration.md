# 企业微信渠道联调指南

> 对应 M4：`hi gateway` — 企业微信智能机器人 WebSocket 长连接  
> 官方文档：[智能机器人长连接](https://developer.work.weixin.qq.com/document/path/101463)

## 前置条件

| 项 | 要求 |
|----|------|
| 网络 | 本机可 **出站** 访问 `wss://openws.work.weixin.qq.com` |
| 企微后台 | 已创建 **智能机器人**（不是旧版自建应用回调） |
| API 模式 | **长连接**（不要填「接收消息 URL」） |
| 凭证 | `bot_id` + `Secret`（长连接专用 Secret） |
| LLM | `~/.hi/hi.toml` 中 AI 已配置（Agent 需要调模型） |

## 一、后台配置（5 分钟）

1. 登录 [企业微信管理后台](https://work.weixin.qq.com/)
2. **应用管理 → 智能机器人 → 创建机器人**
3. **API 模式** 选择 **长连接**
4. 复制 **Bot ID**、**Secret**（仅显示一次，请妥善保存）
5. 发布/启用机器人，确保成员可见

## 二、本地配置

LLM 与消息渠道 **同一文件** `~/.hi/hi.toml`：

| 段 | 用途 |
|----|------|
| `[ai]`、`[workspace]` | LLM、工作目录（`hi setup`） |
| `[channels.wecom]` 等 | 企微 Gateway 渠道（`hi gateway setup`） |

```bash
hi setup              # LLM + workspace
hi gateway setup      # 交互式写入 hi.toml 的 [channels.wecom]
hi config             # 确认路径与脱敏后的内容
```

> 旧版 `[wecom]` 段仍可加载；新配置与向导统一使用 `[channels.wecom]`。

### 单个机器人

编辑 `~/.hi/hi.toml`（或由向导生成）：

```toml
[channels.wecom]
bot_id = "你的BotID"
secret = "你的Secret"
# welcome_message = "你好，我是 hi，极致轻量化的个人 AI 助手"
# dm_policy = "allowlist"   # 默认；须配置 allow_from
# dm_policy = "open"        # 开发联调：所有用户可触发
# allow_from = ["userid"]
```

`secret` 写在 `hi.toml` 中（权限 `600`），**不要**提交到 Git。

### 多个机器人并发（同一 Gateway 进程）

一个 `hi gateway` 进程可同时维持 **多条 WebSocket**（每个 bot 一条连接）。在 `hi.toml` 中为每个 bot 建一个 **命名子段**：

```toml
# 可选：显式指定要启动的 endpoint；省略则自动启用所有已配置账户
enabled = ["wecom", "wecom:support"]

[channels.wecom]
bot_id = "main-bot-id"
secret = "main-secret"

[channels.wecom.support]
bot_id = "support-bot-id"
secret = "support-secret"
```

**endpoint 命名规则：**

| 配置段 | endpoint id | 会话 id（单用户） |
|--------|-------------|-------------------|
| `[channels.wecom]` | `wecom` | `wecom:{userid}` |
| `[channels.wecom.support]` | `wecom:support` | `wecom:support:{userid}` |
| `[channels.wecom.ops]` | `wecom:ops` | `wecom:ops:{userid}` |

不同 bot 之间 **会话历史隔离**；同一 userid 在不同 bot 下也是独立会话。

**`enabled` 与 `default` 的优先级：**

1. 写了 `enabled = [...]` → 只启动列表中的 endpoint
2. 没写 `enabled`、但写了 `default = "wecom"` → **仅启动** `default` 这一个（向导默认行为）
3. 两者都没写 → **自动启动** 所有已配置的 `[channels.wecom]` / `[channels.wecom.*]` 账户

若你手动加了第二个 bot，请 **删除 `default`** 或改成完整的 `enabled` 列表，否则会只跑 `default` 指定的那一个。

```bash
export RUST_LOG=info,hi_gateway=debug   # 联调建议开 debug
```

## 三、Gateway 进程管理

| 命令 | 行为 |
|------|------|
| `hi gateway` | 后台启动（默认） |
| `hi gateway start` | 同上 |
| `hi gateway stop` | 停止 |
| `hi gateway restart` | 重启 |
| `hi gateway status` | 状态 + 将运行的渠道列表 |
| `hi gateway run` | 前台运行（调试，日志写入 `~/.hi/logs/hi.log`） |
| `hi gateway --check` | 预检所有 enabled endpoint 后退出 |

后台运行时：

- PID：`~/.hi/run/gateway.pid`
- 日志：`~/.hi/logs/hi.log`（按日切割，如 `hi.log.2026-06-03`）

## 四、分步验收

### Step 1：编译

```bash
cargo build -p hi
cargo test --workspace
```

### Step 2：连接预检（不常驻）

```bash
hi gateway --check
```

**期望输出（单 bot）：**

```
wecom check: connected
wecom check OK — 订阅成功，可在企微中向机器人发消息
gateway check OK
```

多 bot 时会对 **每个** enabled endpoint 各做一次订阅预检。

**常见失败：**

| 现象 | 原因 | 处理 |
|------|------|------|
| `subscribe failed` errcode≠0 | bot_id 或 Secret 错误 | 核对后台凭证、是否复制完整 |
| `subscribe ack timeout` | 网络/防火墙拦截 wss | 检查公司代理、出站 443 |
| `websocket connect` 失败 | DNS/TLS | 检查出站 HTTPS |
| `wecom.bot_id is empty` | 未写配置 | `hi config` |
| `未配置 wecom 账户 "support"` | `enabled` 与配置段不一致 | 核对 `[channels.wecom.support]` 是否存在 |

### Step 3：启动 Gateway

```bash
hi gateway              # 后台
# 或
hi gateway run          # 前台看日志
```

**期望日志（含 endpoint 字段）：**

```
gateway 已启动 (pid …)
endpoint=wecom wecom AI bot connected
endpoint=wecom wecom subscribed — waiting for messages
# 多 bot 时还会有 endpoint=wecom:support …
```

进程保持运行；每个连接每 30s 发一次 `ping`（带 `req_id`，符合官方协议）。

### Step 4：企微端验证

1. 在企业微信 **单聊** 打开该智能机器人
2. 首次进入应收到欢迎语（`enter_chat` → `aibot_respond_welcome_msg`）
3. 发送：`你好`
4. 应先看到「思考中…」，再收到 Agent 回复（超长内容会分多条消息发送，不截断）

多 bot 时：分别向 **不同机器人** 发消息，确认各自独立回复、会话不串。

### Step 5：会话隔离

```bash
# 另开终端
hi chat
```

| 入口 | 会话 id |
|------|---------|
| 主企微 bot | `wecom:{userid}` |
| 命名 bot（如 support） | `wecom:support:{userid}` |
| `hi chat` | `chat:main` |
| `hi tui` | `tui:main` |

以上 **互不共享** 上下文。验证：`hi config` 查看 `sessions.db` 路径，或用 SQLite 查看 `messages` 表。

### Step 6（可选）：危险命令审批

在企微中让 Agent 执行 bash（如触发 bash 工具）：

- 收到「请回复 Y 执行」
- 回复 `Y` 后继续

### Step 7（可选）：白名单

默认 `dm_policy=allowlist`。生产环境须配置 `allow_from`；`hi gateway --check` 会在 allowlist 为空时报错。

```toml
[channels.wecom]
dm_policy = "allowlist"
allow_from = ["你的userid"]
```

命名 bot 可单独配置策略：

```toml
[channels.wecom.support]
bot_id = "…"
secret = "…"
dm_policy = "allowlist"
allow_from = ["ops-userid"]
```

非白名单用户消息会被静默忽略（日志 `wecom dm blocked by policy`）。

## 五、调试技巧

```bash
# 详细 WebSocket 帧日志
RUST_LOG=debug hi gateway run

# 只看 gateway
RUST_LOG=hi_gateway=debug,info hi gateway run

# 后台日志（按日切割，取最新文件）
tail -f ~/.hi/logs/hi.log*
```

关注日志关键字：

- `endpoint=wecom` / `endpoint=wecom:support` — 区分哪条连接
- `wecom inbound` — 收到的原始帧（debug）
- `wecom message` — 用户文本消息
- `wecom enter_chat` — 进入会话
- `subscribe failed` — 凭证问题
- `wecom disconnected_event` — 可能有新连接顶替（多实例误启）

**注意：** 同一 `bot_id` 不要同时在两个进程里订阅（例如两个 `hi gateway` 或本机 + 另一台机器），企微会踢掉旧连接。

## 六、协议对照（已实现）

| 方向 | cmd | hi 行为 |
|------|-----|---------|
| → 企微 | `aibot_subscribe` | 连接后认证，等待 errcode=0 |
| → 企微 | `ping` | 每 30s 心跳（含 headers.req_id） |
| → 企微 | `aibot_respond_welcome_msg` | enter_chat 欢迎语 |
| → 企微 | `aibot_respond_msg` | 流式回复（finish=false/true） |
| ← 企微 | `aibot_msg_callback` | 用户消息 → Agent |
| ← 企微 | `aibot_event_callback` | enter_chat / disconnected 等 |

## 七、联调 Checklist

- [ ] 后台 API 模式 = **长连接**
- [ ] `~/.hi/hi.toml` LLM 已配置（`hi setup`）
- [ ] `~/.hi/hi.toml` 中 `[channels.wecom]` 的 `bot_id` / `secret` 已填（`hi gateway setup`）
- [ ] 多 bot 时：`enabled` 正确，或已删除仅跑单 bot 的 `default`
- [ ] `hi gateway --check` 成功（每个 endpoint 均通过）
- [ ] `hi gateway` 后台运行，`hi gateway status` 显示运行中
- [ ] 企微单聊收到欢迎语
- [ ] 发文本收到 Agent 回复
- [ ] （多 bot）各机器人回复独立、会话 id 不串
- [ ] `hi chat` 与企微会话不串
- [ ] （可选）allowlist / bash 审批

## 八、仍不支持（后续迭代）

- 图片/文件/语音消息（会回复「暂不支持，请发文本」）
- 群聊 @机器人（需验证 chatid 场景）
- 模板卡片交互
- 主动推送 `aibot_send_msg`
- 非企微平台 — 架构已预留 `ChannelEndpoint`；飞书、个人微信 iLink 已实现，见 [feishu-gateway-integration.md](feishu-gateway-integration.md) · [weixin-ilink-integration.md](weixin-ilink-integration.md)

---

示例配置见仓库根目录 [hi.example.toml](../../hi.example.toml)。
