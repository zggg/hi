# 个人微信 iLink 渠道联调指南

> `hi gateway` — 腾讯 iLink 协议 HTTP 长轮询（实验性）

## 前置条件

| 项 | 要求 |
|----|------|
| 手机微信 | iOS 8.0.70+ / Android 8.0.69+ |
| iLink 插件 | 我 → 设置 → 插件 → 启用 iLink 插件 |
| 网络 | 本机可出站访问 `ilinkai.weixin.qq.com` |
| LLM | `~/.hi/hi.toml` 中 AI 已配置 |

iLink 渠道**仅支持本人私聊**，无群聊、无白名单配置。

## 一、配置与启动

```bash
hi setup
hi gateway setup      # 选「个人微信（iLink）」，终端内扫码
hi gateway --check
hi gateway            # 或 hi gateway run
```

### `~/.hi/hi.toml` 示例

```toml
[channels.weixin]
bot_token = "登录后自动写入"
ilink_bot_id = "..."
ilink_user_id = "xxx@im.wechat"
welcome_message = "你好，我是 hi"
poll_timeout_secs = 35
```

游标文件（自动）：`~/.hi/data/weixin-weixin.json`

## 二、会话

| endpoint | SessionId |
|----------|-----------|
| `weixin` | `weixin:main` |

## 三、危险命令确认

回复「确认」执行，或「取消」放弃。

## 四、排查无回复

```bash
RUST_LOG=info,hi_gateway=debug hi gateway
```

| 现象 | 处理 |
|------|------|
| 无 `weixin inbound message` 日志 | 确认 gateway 在跑；检查 bot_token |
| 有 inbound 无 send | 看 `weixin send failed` 错误 |
| token 失效 | 重新 `hi gateway setup` 扫码 |
