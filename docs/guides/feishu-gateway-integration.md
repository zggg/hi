# 飞书渠道联调指南

> `hi gateway` — 飞书机器人长连接（WebSocket）  
> 官方文档：[使用长连接接收事件](https://open.feishu.cn/document/server-docs/event-subscription-guide/event-subscription-configure-/request-url-configuration-case)

## 前置条件

| 项 | 要求 |
|----|------|
| 应用类型 | **企业自建应用**（长连接不支持商店应用） |
| 网络 | 本机可 **出站** 访问 `open.feishu.cn` |
| 机器人 | 应用已开启 **机器人** 能力 |
| 事件 | 订阅 `im.message.receive_v1`（接收消息） |
| 订阅方式 | **使用长连接接收事件**（无需公网 URL） |
| LLM | `~/.hi/hi.toml` 中 AI 已配置 |

## 一、飞书开放平台配置

1. 登录 [飞书开放平台](https://open.feishu.cn/app) 创建企业自建应用
2. **凭证与基础信息** 复制 **App ID**、**App Secret**
3. **应用能力 → 机器人** 启用机器人
4. **权限管理** 开通（按使用场景）：
   - 单聊：`im:message.p2p_msg:readonly` 或 `im:message.p2p_msg`
   - 群聊 @ 机器人：`im:message.group_at_msg:readonly`
   - 群聊全部消息（`mention_enabled=false` 时）：`im:message.group_msg`（敏感权限）
   - 发消息：`im:message` 或 `im:message:send`
5. **事件与回调 → 事件配置**：
   - 添加事件 `im.message.receive_v1`
   - 订阅方式选 **使用长连接接收事件**
   - 先启动 `hi gateway` 或 `hi gateway --check` 建立长连接后，再在后台保存
6. **版本管理与发布** 创建版本并发布（测试企业可先发布到测试版）
7. 将机器人 **加入目标群聊**（群设置 → 群机器人 → 添加）

## 二、本地配置

```bash
hi setup              # LLM + workspace
hi gateway setup      # 交互式写入 [channels.feishu]
hi gateway --check    # 预检 Token + 长连接
hi gateway            # 启动网关
```

### 配置示例（`~/.hi/hi.toml`）

```toml
[channels.feishu]
app_id = "cli_xxxxxxxx"
app_secret = "你的App Secret"
# domain = "open.larksuite.com"   # 国际版 Lark 可选
dm_policy = "allowlist"
allow_from = ["ou_xxxxxxxx"]      # 用户 open_id
mention_enabled = true            # 群聊需 @机器人；false 则响应群内所有文本
# welcome_message = "你好，我是 hi"
```

### `mention_enabled` 行为

| 场景 | `mention_enabled = true`（默认） | `mention_enabled = false` |
|------|----------------------------------|---------------------------|
| 私信 Bot | ✅ 白名单用户可直接对话 | ✅ 同上 |
| 群聊 | 仅响应 **@机器人** 的消息 | 响应群内 **所有文本** 消息 |
| 所需权限 | `im:message.group_at_msg:readonly` | `im:message.group_msg`（敏感） |

### 用户 open_id

- 开发调试：在飞书开放平台 **事件日志** 查看 `sender_id.open_id`
- 生产环境：通过通讯录 API 或首次对话后从日志获取

## 三、会话隔离

| 配置段 | endpoint id | 会话 id |
|--------|-------------|---------|
| `[channels.feishu]` | `feishu` | `feishu:{open_id}` |
| `[channels.feishu.ops]` | `feishu:ops` | `feishu:ops:{open_id}` |

同一用户在不同群聊中共享同一会话（按 open_id 隔离，不按 chat_id）。

## 四、常见问题

| 现象 | 排查 |
|------|------|
| 后台保存长连接失败 | 先运行 `hi gateway --check` 确保客户端在线 |
| 群聊无响应 | 检查 `mention_enabled`、是否 @ 了机器人、群消息权限是否开通并发版 |
| 私信无响应 | 检查 `allow_from` 是否包含发送者 open_id |
| 收消息但发不出 | 检查 `im:message` 发送权限、应用是否已发布 |
| 连接断开 | Gateway 会自动指数退避重连；查看日志 `feishu gateway disconnected` |

## 五、与企业微信并行

`hi gateway` 可同时启动企微与飞书（各自独立 WebSocket + endpoint）：

```toml
[channels.wecom]
bot_id = "..."
secret = "..."

[channels.feishu]
app_id = "cli_..."
app_secret = "..."
allow_from = ["ou_..."]
```
