# HTTP Gateway API

`hi gateway` 默认启动 API Server（`127.0.0.1:9527`），会话 id 由 URL 指定，服务端映射为 `http:{id}`。

## 鉴权

除 `GET /healthz` 外，请求需携带：

```http
Authorization: Bearer <token>
```

token 来自 `~/.hi/hi.toml` 的 `[channels.http].token`。首次 `hi gateway` 若 token 为空会自动生成 64 字符 hex 并写回配置。

## 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/healthz` | 健康检查（无需鉴权） |
| GET | `/v1/info` | 当前 provider / model / locale |
| GET | `/v1/sessions` | 会话列表 |
| GET | `/v1/sessions/{id}` | 会话 transcript |
| POST | `/v1/sessions/{id}/turns` | 发起回合；默认 SSE 流式 `AgentEvent` |
| POST | `/v1/sessions/{id}/approvals` | body `{"approved": true\|false}`，回应 bash 审批 |

## 流式回合

```sh
curl -N -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"message":"你好"}' \
  http://127.0.0.1:9527/v1/sessions/alice/turns
```

SSE 每条 `data:` 行是一个 `AgentEvent` JSON，以 `turn_completed` 结束。

## 非流式 JSON

```sh
curl -H "Authorization: Bearer <token>" \
  -H "Accept: application/json" \
  -H "Content-Type: application/json" \
  -d '{"message":"你好"}' \
  http://127.0.0.1:9527/v1/sessions/alice/turns
```

## 配置

```toml
[channels.http]
enabled = true
host = "127.0.0.1"
port = 9527
token = ""   # 空则首启自动生成

[gateway]
max_concurrent_turns = 16   # 与 IM 渠道共用全局并发上限
```

- 改 `token` 后 `hi gateway reload`（SIGUSR1）即时生效
- 改 `host` / `port` 需 `hi gateway restart`
