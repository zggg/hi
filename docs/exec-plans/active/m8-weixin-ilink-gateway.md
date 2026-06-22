# M8：个人微信 iLink Gateway

This ExecPlan is a living document.

## Purpose / Big Picture

`hi gateway` 通过腾讯官方 **iLink 协议** 接入个人微信 iLink 插件，实现私聊收发消息。

- 直连 `ilinkai.weixin.qq.com`
- HTTP 长轮询，无需公网 URL
- 实验性渠道：灰度功能，条款允许腾讯随时调整

详细设计见 [2026-06-09-weixin-ilink-gateway-design.md](../../design/2026-06-09-weixin-ilink-gateway-design.md)。

## Progress

### Phase 1 — MVP

- [x] `core/src/config/weixin.rs` — `WeixinConfig`
- [x] `ChannelsConfig` / `ChannelEndpoint` / `Channel::Weixin` 扩展
- [x] `gateway/src/weixin/` — `IlinkClient` + `WeixinAdapter` + 长轮询
- [x] `gateway_channel.rs` 注册 `weixin`（实验性）
- [x] `app/src/config/gateway.rs` — setup 扫码向导
- [x] `memory/owner.rs` — `weixin:` 前缀
- [x] `hi gateway --check` 预检
- [x] 联调文档 `docs/guides/weixin-ilink-integration.md`
- [ ] 真实 iLink 环境联调（需用户手机灰度 + 扫码）

### Phase 1.1 — 体验

- [x] `sendtyping` 正在输入
- [x] `~/.hi/data/weixin-{id}.json` 游标持久化
- [x] 危险命令文本确认回复（确认 / 取消）

### Phase 2 — 扩展

- [ ] 图片/文件（CDN + AES）
- [ ] 群聊（视官方能力）

## Key Files

```
core/src/config/weixin.rs          # 新增
core/src/config/channels.rs        # weixin_accounts
core/src/config/endpoint.rs        # Weixin variant
core/src/channel.rs                # Channel::Weixin
core/src/memory/owner.rs           # weixin: 前缀
gateway/src/weixin/                # 新增目录
gateway/src/adapter.rs             # build_adapter 分支
app/src/config/gateway.rs          # 向导
```

## Validation

```sh
# 前置：手机微信 8.0.70+，插件中已启用 iLink 插件
hi setup
hi gateway setup                   # 选择「个人微信（iLink）」，扫码
hi gateway --check
RUST_LOG=info,hi_gateway=debug cargo run -p hi -- gateway

# 另开终端
hi session show --session weixin:<sender_id>
hi config                          # 确认 token 脱敏
```

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
./scripts/check-consistency.sh
```

## Risks

- 灰度未开放 → 向导文档说明，无法联调属预期
- `bot_token` 失效 → 提示重新 `hi gateway setup`
- 协议非正式文档 → `IlinkClient` 隔离，便于后续切换
