# Reliability（服务可靠性）

hi 含 HTTP Gateway 组件，以下为计划中的可靠性约定。

## 进程模型（v1）

| 命令 | 进程 | 说明 |
|------|------|------|
| `hi tui` | 单进程 | 本地交互 |
| `hi gateway` | 单进程 | axum 监听 `127.0.0.1:8787` |
| 并发运行 | 两进程 | 共享 SQLite WAL |

## Gateway 韧性（计划 M4）

- 长任务先回复「思考中…」
- 渠道回复超长时分片发送（`channel_reply_chunks`），禁止截断
- 回调处理超时应有日志与 tracing span
- 企微 API 失败时重试策略（有限次数 + 退避）

## 数据一致性

- SQLite WAL 模式 + 短事务
- 极端情况：session 级 mutex（设计文档预留）

## 错误预算

v1 无正式 SLA；企微走 WebSocket 出站长连接，无内网穿透依赖。

## 可观测性

- 当前：`tracing` 结构化日志
- 未做：metrics、分布式追踪
