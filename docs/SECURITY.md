# Security

## Authentication

| Flow | Method | Where |
|------|--------|-------|
| 本地 TUI | 无远程认证（单用户本机） | `hi tui` 进程 |
| 消息渠道 Gateway | 渠道侧鉴权 + 用户白名单（`dm_policy` + `allow_from`） | `hi-gateway` |
| LLM API | Provider API Key | `~/.hi/hi.toml` 的 `[ai.providers.<name>].api_key` |

v1 为单用户场景。企微/飞书默认 `dm_policy=allowlist` + `allow_from`；显式设为 `open` 时所有用户可触发 Agent（启动时会 warn）。个人微信 iLink 仅支持本人私聊，无白名单配置。

## Authorization

- Gateway 收到消息后校验发送者是否在白名单内，拒绝未授权用户
- `bash` 工具执行前需用户审批（TUI 弹窗 / Gateway 回复确认）
- 统一审批策略 `[tools.approvals]`（`hi-core/src/approval/policy.rs`）覆盖 bash + 文件工具
- `mode = off` 全部免审；`workspace.trust = true` 时 workspace 内 read/write/edit/bash 免审（hardline 与 deny 除外）
- 危险 bash 命令需确认；同意后写入 `commands.allow`（按 grant_key 精确匹配，非子串）
- 命令检测前会做 deobfuscate（反斜杠/空引号、`eval`/`command` 前缀、`$()`/`echo` 展开、`${IFS}` 等）；未收敛时命中 `obfuscated-shell`
- 常见 grant_key：`sudo`、`curl`、`wget`、`eval`、`rm`、`bash`/`sh -c`、`pipe-to-shell`（含 `base64 -d | bash`）、`obfuscated-shell`
- hardline（`rm -rf /`、fork bomb、`> /dev/*`、`mkfs`、`dd`、`chmod 777` 等）永远不可 grant
- bash 写盘（`>`/`cp`/`mv`/`tee`/`sed -i`/`dd of=` 等）与 read/write/edit 共用 `filesystem` 规则

授权逻辑在 `hi-core` 工具层与 `hi-gateway` 入口层执行，见 [LAYERS.md](architecture/LAYERS.md)。

## Gateway 生产约束

- **共享持久化**：`HiServices` 持有单一 `SessionStore`（SQLite WAL），禁止每回合新建连接
- **会话串行**：`SessionCoordinator` 按 `session_id` 串行 agent turn，避免同用户并发写库
- **全局背压**：Gateway 使用 semaphore 限制并发 turn 数（默认 16）
- **日志脱敏**：企微消息内容仅记录长度（`debug!(len=…)`），不记录全文

## Secrets Management

- **Storage:** `~/.hi/hi.toml`（`[ai.providers.*].api_key`、`[channels.wecom].secret` 等）；保存时 Unix 下 `chmod 600`
- **Rotation:** 手动编辑配置并轮换密钥
- **Access:** 仅本机 hi 进程读取；`hi config` 脱敏显示；勿将 `hi.toml` 提交 git

## Threat Model

| Threat | Mitigation | Status |
|--------|-----------|--------|
| 未授权渠道用户触发 Agent | `dm_policy` + `allow_from`（默认 allowlist） | M4 ✅ |
| 同用户并发写 SQLite | `SessionCoordinator` per session | ✅ |
| 危险 shell 命令 | bash 审批 + hardline 不可 grant；`commands.allow` 可免审 | M2 ✅ |
| 范围外文件访问 | `filesystem.allow_*` 前缀匹配 + bash 重定向；用户确认后持久化 | ✅ |
| 回调伪造 | 企微 WebSocket 长连接 + 渠道 Token | M4 ✅ |
| API Key 泄露 | 配置文件权限 + `hi config` 脱敏；禁止写入日志 | in place |
| 旧版 sessions.db | `schema.ensure_compatible` 拒绝不兼容库，提示删库重建 | ✅ |

## Dependencies

- 安全相关：`serde`（配置）、`tokio`（异步）、`reqwest`（LLM / 企微 API）
- 依赖更新：手动 `cargo update`；建议定期 `cargo audit`

## Incident Response

- 泄露 API Key：立即轮换 Provider 密钥，检查日志是否有异常调用
- 渠道连接异常：检查渠道凭证与网络，临时关闭 `hi gateway` 进程
- 数据库 schema 不兼容：备份后删除 `~/.hi/data/sessions.db`，重新运行 `hi` 自动重建

## 漏洞报告

请勿在公开 Issue 中披露可利用的安全问题。请通过 GitHub **Security Advisories**（仓库 → Security → Report a vulnerability）私下报告，或在仓库启用 Issues 前通过维护者邮箱联系。

报告请包含：影响版本、复现步骤、预期与实际行为。我们会在确认后尽快回复。

## 第三方服务

hi 可对接企微、飞书、微信 iLink、各 LLM Provider 及本地 Codex 凭证。这些服务的可用性、条款与合规责任由用户自行承担；hi 项目不对第三方 API 变更或账号限制负责。
