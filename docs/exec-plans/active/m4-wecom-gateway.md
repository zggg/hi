# M4：消息渠道 Gateway（企业微信）

This ExecPlan is a living document.

## Purpose / Big Picture

`hi gateway` 通过企业微信官方 **智能机器人长连接** 收发消息（首个渠道实现；后续可接更多平台）。

- 仅需 `bot_id` + `secret`
- 无需公网 URL、ngrok/frp、Token、EncodingAESKey

## Progress

- [x] WebSocket `aibot_subscribe` / `aibot_msg_callback` / `aibot_respond_msg`
- [x] 订阅 ack 等待 + errcode 校验
- [x] 官方格式心跳 `ping`（含 req_id）
- [x] `enter_chat` 欢迎语
- [x] `hi gateway --check` 预检命令
- [x] 联调文档 `docs/guides/wecom-gateway-integration.md`
- [ ] 真实企微环境联调（需用户凭证）

## Validation

```sh
export WECOM_SECRET=...
export RUST_LOG=info,hi_gateway=debug

cargo run -p hi -- gateway --check
cargo run -p hi -- gateway
```

企微后台：API 模式 → **长连接**。

详细步骤见 [wecom-gateway-integration.md](../guides/wecom-gateway-integration.md)。
