# M11：HTTP 接口（gateway endpoint）

> 编号说明：**M10** 已由 [m10-gateway-common.md](m10-gateway-common.md)（gateway 公共抽象，已完成）占用；本计划为 **M11**。

This ExecPlan is a living document.

## Purpose

- 把 `hi` 的 Agent 能力通过 **HTTP + SSE** 暴露给前端/编程客户端
- **不新增 crate**：作为 `hi-gateway` 的一种 endpoint，随 `hi gateway` 一起跑
- 协议建模 hi 领域：**会话**（`http:{id}`）+ **回合**（turn）+ **AgentEvent** 流式事件
- 复用 gateway 既有公共设施（审批总线、并发限流、去重、turn 管线）

## 已锁定的设计决定

| # | 决定 | 取值 |
|---|------|------|
| 1 | 放置方式 | `ChannelEndpointKind::Http` + `HttpAdapter: ChannelAdapter`，随 `hi gateway` 启动 |
| 2 | 会话隔离 | `http:{id}`，**id 必填**，每个前端/用户各自独立会话 |
| 3 | 会话只读接口 | **做**，提取抽象 `SessionReader` trait（不污染 `PersistedAgentHost`） |
| 4 | HTTP 框架 | **axum** |
| 5 | `[channels.http]` | **默认开启**（`enabled` 缺省为 `true`） |
| 6 | token | 首次启动若为空则**随机生成并持久化**；用户手动设置后不覆盖 |
| 7 | reload | `hi gateway reload`（SIGUSR1）需重载 `[channels.http]` 运行期字段（token / 鉴权开关） |

## 架构定位：HTTP = 又一种 endpoint

复用现有分发链 `enabled_endpoints() → build_adapter() → adapter.run()`，`gateway/src/run.rs` 无需改动：

```
ChannelsConfig.enabled_endpoints()   # 默认包含 "http"
  └─ build_adapter(Http{config})  →  HttpAdapter
       └─ run()  →  起 axum server，阻塞（与 WeCom/Feishu 任务并排 spawn）
```

`ChannelAdapter` 的 `name / check / run` 三件套对 HTTP 同样成立：
- `name()` → `"http"`
- `check()` → 校验端口可绑定、token 非空（`hi gateway --check` 自动覆盖）
- `run()` → 启动 axum，持续服务直到进程退出

## HTTP 协议（会话优先 + 流式 AgentEvent）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/v1/sessions/{id}/turns` | body `{"message": "...", "workdir": "?"}`；SSE 逐条吐 `AgentEvent`，以 `turn_completed` 收尾 |
| POST | `/v1/sessions/{id}/approvals` | body `{"approved": true}`；回应流中的 `approval_required` |
| GET | `/v1/sessions` | 会话列表（经 `SessionReader`） |
| GET | `/v1/sessions/{id}` | 某会话历史 transcript（经 `SessionReader`） |
| GET | `/v1/info` | 当前 model / provider / locale |
| GET | `/healthz` | 健康检查（无需鉴权） |

> `{id}` 必填，服务端拼成 `Channel::Http` → `http:{id}`。不带 id 的请求 → 404（路由不匹配），从协议层杜绝串台。

### SSE 事件 = 直接序列化 `AgentEvent`

`AgentEvent` 已是 `#[serde(tag="type", rename_all="snake_case")]`，直接 `data: {json}\n\n`。核心处理器：

```rust
let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
let host = state.host.clone();
tokio::spawn(async move { host.run_turn(sid, wd, &msg, &approval, Some(tx)).await });
// axum SSE：每个 AgentEvent → 一条 data 行
while let Some(ev) = rx.recv().await {
    yield Event::default().data(serde_json::to_string(&ev)?);
}
```

客户端收到的 SSE 载荷即 `AgentEvent` 序列，例如：`assistant_delta`、`reasoning_delta`、`tool_call_started`、`tool_call_finished`、`tool_output_delta`、`file_diff`、`approval_required`、`context_compressed`、`knots_injected`、`turn_completed`、`error`。

## 会话只读抽象：`SessionReader`

`PersistedAgentHost` 只有 `run_turn`，拿不到 `SessionStore`。新增**只读** trait（core 定义，`HiServices` 实现），映射现有 `SessionStore` 能力：

```rust
// core/src/agent_host.rs（或新 core/src/session_reader.rs）
#[async_trait]
pub trait SessionReader: Send + Sync {
    fn list_sessions(&self) -> Result<Vec<SessionSummary>>;          // → store.list_sessions()
    fn load_all_messages(&self, id: &SessionId) -> Result<Vec<StoredMessage>>; // → store.load_all_messages()
}
```

gateway 需要的 host 能力收敛成一个超 trait，`run_gateway` / `build_adapter` 改用它（IM adapter 只用到 `run_turn` 部分）：

```rust
pub trait GatewayHost: PersistedAgentHost + SessionReader {}
impl<T: PersistedAgentHost + SessionReader> GatewayHost for T {}
```

`HiServices` 已实现 `PersistedAgentHost`，只需补 `SessionReader`（它本就持有 `Arc<SessionStore>`），自动获得 `GatewayHost`。**只读**：DELETE/purge 等写操作留到后续（不在本期）。

## 配置：`[channels.http]`

```toml
[channels.http]
enabled = true        # 缺省 true（决定 5）
host = "127.0.0.1"    # 默认仅回环
port = 9527
token = ""            # 空 → 首次启动随机生成并写回（决定 6）
```

- 新增 `core/src/config/http.rs` → `HttpConfig`，与其它渠道一致提供 `is_empty()`。
- 复用 `parse_platform_accounts` 把 HTTP 当"只有 default 账号的平台"接入；命名账号（`[channels.http.admin]`）→ 多监听端口白送。
- `ChannelsConfig` 的 `all_enabled_endpoint_ids` / `endpoint_by_id` / `save` / `Serialize` 各加一支 http。

### 默认开启的连带影响

`enabled_endpoints()` 现在**总会**至少含 `http`，因此即使没有任何 IM 渠道，`hi gateway` 也会启动（仅监听回环的 HTTP）。这是决定 5 的预期行为；用户可 `enabled = false` 退出。文档需更新此说明。

### token 自动生成

- 触发点：gateway 启动（前台 `gateway run` 入口）加载 `ChannelsConfig` 后，若 `http.enabled && http.token` 为空 → 用 `getrandom` 取 32 字节 hex 编码（64 字符），写回 `~/.hi/hi.toml`，并在日志打印一次（INFO）方便用户取用。
- 仅当 token 为空时生成；非空一律不动（决定 6）。
- `hi setup` / `hi gateway setup` 向导后续可加"显示当前 HTTP token"。

## Reload 语义（SIGUSR1）

现状：`spawn_gateway_reload_listener` 收 SIGUSR1 → `HiServices::reload_from_disk()`（重载 `[ai]`、`[tools.approvals]`）。本期扩展：

- **可热重载**：`[channels.http]` 的 `token` 与鉴权开关。做法：`HttpAdapter` 的鉴权读取共享 `Arc<RwLock<HttpRuntime>>`，`reload_from_disk` 一并刷新它 → 改 token 后 `hi gateway reload` 即时生效，无需重启、不断开在途连接。
- **需重启**：`host` / `port`（绑定期固定）、`enabled` 由开→关的下线，改动后需 `hi gateway restart`。文档明确标注。

## 审批：复用 gateway 的 `ApprovalBus`

直接复用 `gateway/src/common/approval.rs` 的 `ApprovalBus`（`oneshot` 等待 + 文本 resolve）：

1. bash 触发审批 → `run_turn` 发出 `AgentEvent::ApprovalRequired` → 进入 SSE 流；
2. 客户端 `POST /v1/sessions/{id}/approvals {"approved": true}` → 调 `bus.try_resolve(user_key, "确认"/"取消", true)` 唤醒等待的回合。

`user_key` 用 `http:{id}`，与 IM 渠道同构。`mode = "off"` 时无审批，直接放行（与全局策略一致）。

## 并发模型（多用户）

HTTP **支持多用户/多前端并发**；隔离与限流沿用现有 runtime。

| 层级 | 行为 | 来源 |
|------|------|------|
| **传输** | 每个 HTTP 请求独立 async task，并行接受连接 | axum / tokio |
| **会话** | 不同 `http:{id}` **并行**跑回合；同一 id **串行**（防 transcript 乱序） | `SessionCoordinator::with_session` |
| **全局 turn** | 最多 **N** 路 agent 回合同时执行（默认 16），超出排队；企微/飞书/HTTP 共用同一 Semaphore | `[gateway].max_concurrent_turns` |
| **SQLite** | `SessionStore`：1 写 + 读池（`[storage].read_pool_size`，全入口共享） | [M12](m12-sqlite-concurrent-access.md) |

**典型场景**

- 用户 A（`http:alice`）与用户 B（`http:bob`）同时聊天 → 两路 turn 并行，各自独立会话历史。
- 同一用户连发两条 → 第二条等第一条 `turn_completed` 后再跑（同 session gate）。
- 多 session 并行 turn + 旁路读历史 → turn 并行；旁路读走读池，写 transcript 走写连接（M12）。

## 复用清单（gateway 公共设施）

| 设施 | 来源 | 用途 |
|------|------|------|
| `ApprovalBus` / `ChannelApproval` | `common::approval` | 交互式 bash 审批 |
| 并发限流 | `common::concurrency` + 全局 `Arc<Semaphore>` | 与 IM 共享 `[gateway].max_concurrent_turns` |
| 去重 | `common::dedup` | `Idempotency-Key` |
| `run_agent_turn` | `common::turn` | 非流式响应（含重试/超时/dead-letter） |
| `channel_reply_text` | `hi-core` | 非流式时抽最终回复 |
| `SessionCoordinator` | `HiServices` | 同一 `http:{id}` 回合串行化 |

## 鉴权与安全

- `Authorization: Bearer <token>`；token 来自 `[channels.http].token`。
- 默认仅绑 `127.0.0.1`；绑非回环地址且 token 为空 → `check()` 报错拒绝启动。
- 沿用 hi 全局审批策略（hardline 命令始终拦截，不受 token / mode 影响）。
- 响应加 `X-Content-Type-Options: nosniff`。

## 依赖

- `axum`（含 SSE）+ `tower`（axum 传递依赖）→ 加到 `gateway/Cargo.toml`。
- `getrandom`（token 生成，极小；rustls 依赖树已含，提升为直接依赖）。

## Key Files

```
core/src/config/http.rs        # HttpConfig（新）
core/src/config/endpoint.rs    # ChannelEndpointKind::Http（改）
core/src/config/channels.rs    # http 接入 enabled_endpoints / save / serialize（改）
core/src/channel.rs            # Channel::Http + http_session(id)（改）
core/src/session_reader.rs     # SessionReader trait（新）；或并入 agent_host.rs
core/src/agent_host.rs         # GatewayHost 超 trait（改）
gateway/src/http/              # mod / adapter / server / routes / sse / approval（新）
gateway/src/adapter.rs         # build_adapter 增 Http 分支（改）
gateway/src/run.rs             # host 类型 → Arc<dyn GatewayHost>（改）
gateway/src/lib.rs             # pub mod http（改）
app/src/services.rs            # impl SessionReader；reload_from_disk 刷新 http 运行期（改）
app/src/main.rs                # 启动时 token 自动生成 + 持久化（改）
gateway/Cargo.toml             # axum / getrandom（改）
docs/、AGENTS.md、ARCHITECTURE.md  # 渠道列表 + 默认开启说明（改）
```

> 不动 `docs/architecture/LAYERS.md` 与边界测试——无新增 crate，依赖矩阵不变。

## 循环目标（每 Phase 结束必跑）

```sh
cargo test --workspace
cargo clippy --workspace -- -D warnings
./scripts/check-consistency.sh
# 手工冒烟：
hi gateway run &                                  # 默认起 HTTP，日志打印随机 token
curl -N -H "Authorization: Bearer <token>" \
  -d '{"message":"你好"}' \
  http://127.0.0.1:9527/v1/sessions/alice/turns   # SSE 事件流
```

| Phase | 循环验收 |
|-------|----------|
| **0** | `HttpConfig` + `[channels.http]` 解析/保存；`Channel::Http`/`http_session`；`enabled_endpoints` 含 http；单测通过 |
| **1** | `HttpAdapter` 起 axum；`POST /turns` SSE 跑通；`GET /healthz`/`/v1/info`；Bearer 鉴权 |
| **2** | `SessionReader` + `GatewayHost`；`GET /v1/sessions`、`GET /v1/sessions/{id}` |
| **3** | 交互式审批：`approval_required` 入流 + `POST /approvals` 复用 `ApprovalBus` |
| **4** | token 首启随机生成 + 持久化；SIGUSR1 热重载 token；host/port 改动提示需 restart |
| **5** | 复用 dedup（Idempotency-Key）/ 并发限流；非流式 `Accept: application/json` 分支 |
| **6** | 文档：api 使用指南、`[channels.http]` 默认开启说明、AGENTS/ARCHITECTURE 渠道表 |

## Progress

- [x] Phase 0 — 配置与会话命名
- [x] Phase 1 — axum adapter + turns SSE + 鉴权
- [x] Phase 2 — SessionReader 只读接口
- [x] Phase 3 — 交互式审批
- [x] Phase 4 — token 生成 + 热重载
- [x] Phase 5 — 去重/并发/非流式
- [x] Phase 6 — 文档

## 风险与未决

- **默认开启的语义变更**：`hi gateway` 即使无 IM 渠道也会启动 HTTP。需在 release notes / 文档显著说明。
- **会话只读暂不含写**：DELETE/purge 会话不在本期；如需，后续给 `SessionReader` 加写能力或单开 trait。
- **host/port 不可热改**：仅 token 热重载；端口变更须 restart。已在文档约定。
- **`getrandom` vs 其它随机源**：如不愿加直接依赖，可改读 `/dev/urandom`（仅 Unix）；当前推荐 `getrandom`。
- **SessionStore**：M12 已落地 1 写 + 读池；见 [M12](m12-sqlite-concurrent-access.md)。
