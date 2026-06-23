# hi 设计文档

> 日期：2026-05-22  
> 状态：**历史设计快照** — 正文含早期 `config.toml` / `[wecom]` / `WECOM_SECRET` 等表述；**以 `~/.hi/hi.toml`、`[channels.*]`、`[ai.providers.*]` 及对应 exec-plan 为准**。见文末「演进说明」。
> 项目名：`hi`  
> 定位：极致轻量化的个人 AI 助手  
> 运行方式：`hi tui` | `hi gateway` | `hi config`

## 1. 背景与目标

### 1.1 动机

构建一个 **自研、Rust 实现的极致轻量化个人 AI 助手**：

- **简洁内核**：最小工具集、可扩展、Agent 核心与 UI/渠道解耦
- **多渠道入口**：本地 TUI + 消息 Gateway，共用同一 Agent 运行时；企微为首个渠道，后续并列扩展

### 1.2 已确认需求


| 维度     | 决策                                                             |
| ------ | -------------------------------------------------------------- |
| 产品形态   | 本地 TUI + 远程渠道，共用 Agent 核心                                      |
| 首个渠道   | 企业微信（官方 API）                                                   |
| 技术栈    | Rust，Cargo Workspace                                           |
| MVP 范围 | 4 工具 + SQLite 会话 + 多模型 + 上下文压缩 + 危险命令确认                        |
| 部署     | 本机运行；企微走官方 WebSocket 长连接，无需公网回调                                |
| LLM    | 统一 Provider 抽象，配置文件切换（OpenAI 兼容优先）                             |
| 会话模型   | 按 **渠道 + 会话键** 隔离（`tui:main` / `chat:main` / `wecom:{userid}`） |
| 用户范围   | v1 仅单用户（配置白名单），架构预留多人扩展                                        |
| 演进路径   | 方案 B（Workspace 分层）→ 预留方案 C（Daemon + 客户端）                       |


### 1.3 非目标（MVP 不做）

- Skills 系统、MCP 集成、子 Agent、Cron 定时任务
- 个人微信（无官方 Bot API）
- 多租户 SaaS、完整审计后台

---

## 2. 架构概览

### 2.1 分层结构（5 个 crate）

```
┌─────────────────────────────────────────────────────────────┐
│                     hi (app/)                                │
│   hi tui          hi gateway         hi config               │
│   CLI 入口 + 组装                                             │
└──────────────┬──────────────────────────┬───────────────────┘
               │                          │
               ▼                          ▼
┌──────────────────────────┐   ┌──────────────────────────────┐
│  hi-tui (tui/)           │   │  hi-gateway (gateway/)      │
│  终端 UI                 │   │  消息渠道（当前：企微）       │
└──────────────┬───────────┘   └──────────────┬───────────────┘
               │                              │
               └──────────────┬───────────────┘
                              ▼
               ┌──────────────────────────────┐
               │  hi-core (core/)              │
               │  AgentLoop · tools · store    │
               │  （内部 mod，类似 Java 包）    │
               └──────────────┬───────────────┘
                              ▼
               ┌──────────────────────────────┐
               │  hi-ai (ai/)                 │
               └──────────────────────────────┘
```

`hi-core` 内部模块（一个 JAR 里的多个 Java 包）：

```
core/src/
  config.rs, event.rs, session.rs   # agent 运行时
  tools/                            # read / write / edit / bash
  store/                            # SQLite 会话
```

### 2.2 设计原则

1. **平台无关 core**：TUI 与 Gateway 只做 I/O，不含 Agent 业务逻辑
2. **会话按渠道隔离**：`SessionId` 含渠道前缀，不同入口不共享历史
3. **依赖方向**：`hi → {hi-tui, hi-gateway} → hi-core`；`hi-ai` 为叶子 crate，由 `hi` 组装进 AgentLoop
4. **少 crate、多 mod**：tools / store 在 `hi-core` 内用模块划分，不单独成包
5. **可演进**：core 对外暴露稳定 API，未来 `hi daemon` 复用同一 `AgentLoop`

### 2.3 进程模型（v1）


| 命令           | 进程  | 说明                                                  |
| ------------ | --- | --------------------------------------------------- |
| `hi tui`     | 单进程 | 本地交互，读写 `~/.hi/data/sessions.db`                    |
| `hi gateway` | 单进程 | 出站 WebSocket 连接企微 `wss://openws.work.weixin.qq.com` |
| 两者同时运行       | 两进程 | 共享 SQLite；各自 `SessionId` 独立                         |


> v2 可选：`hi daemon` 独占 AgentLoop，TUI/Gateway 变 RPC 客户端（方案 C）。

---

## 3. Crate 职责（5 个）

### 3.1 `hi-core`（Agent 运行时 + tools + store）

一个 crate，内部按模块组织（类似 Java 里一个 JAR 下的 `com.hi.core.tools` 等）：


| 模块    | 路径                                        | 职责                                        |
| ----- | ----------------------------------------- | ----------------------------------------- |
| agent | `config`, `event`, `session`, `AgentLoop` | 循环、配置、事件、用户/会话 ID                         |
| tools | `core/src/tools/`                         | read / write / edit / bash，`ToolRegistry` |
| store | `core/src/store/`                         | SQLite 会话 CRUD、跨进程共享                      |


- `AgentLoop`：LLM ↔ Tool 循环（ReAct 模式）
- `AgentEvent`：流式事件（供 TUI/Gateway 订阅）
- 上下文压缩触发逻辑

MVP 四个工具（`core/src/tools/`）：


| 工具      | 作用                    |
| ------- | --------------------- |
| `read`  | 读文件（带 path 校验）        |
| `write` | 写文件                   |
| `edit`  | 基于 search/replace 的编辑 |
| `bash`  | 执行 shell（需审批）         |


### 3.2 `hi-ai`（LLM Provider）

- `AiProvider` trait
- 内置适配器：
  - `OpenAiCompatProvider`（覆盖 OpenAI、DeepSeek、Moonshot 等）
  - `AnthropicProvider`（后续）
  - `OllamaProvider`（后续）
- 流式 completion（MVP 可先非流式，TUI 模拟流式）

### 3.3 `hi-tui`（终端 UI）

- 基于 **ratatui** + **crossterm**
- 布局：消息区 + 输入区 + 状态栏（模型、token、cwd）
- 订阅 `AgentEvent` 流式渲染
- 危险命令 `ApprovalRequired` 弹窗（y/n）

### 3.4 `hi-gateway`（消息渠道）

- `ChannelAdapter` trait，按平台并列扩展
- MVP：企微智能机器人 WebSocket（`aibot_subscribe` / `aibot_msg_callback` / `aibot_respond_msg`）
- 访问控制：`dm_policy`（`open` | `allowlist`）+ `allow_from`
- 后续：在同一 crate 内增加其他渠道适配器

### 3.5 `hi`（CLI，目录 `app/`）

- clap 子命令：`tui` | `gateway` | `config`
- 组装 `hi-core` + `hi-ai` + `hi-tui` / `hi-gateway`
- 加载配置、初始化 tracing
- 对外二进制名：`hi`

---

## 4. Agent 循环

### 4.1 流程

```
用户消息
  → 从 store 加载历史
  → 检查 context 长度 → 必要时压缩
  → 构造 messages + tool schemas
  → AiProvider.complete (tool_calls?)
  → 对每个 tool_call:
       bash? →  emit ApprovalRequired → 等待确认
       执行 tool → 结果 append 到 messages
  → 循环直到 assistant 无 tool_calls
  → 持久化到 store
  → emit TurnCompleted
```

### 4.2 上下文压缩

- 阈值：可配置，默认 context 的 80%
- 策略：保留 system + 最近 N 轮 + 中间摘要（lossy summarization）
- 压缩后新建 `SessionId` 子 lineage（可选，v2）

### 4.3 危险命令确认

- `bash` 执行前匹配规则：`rm -rf`、`sudo`、`curl | sh` 等
- TUI：ratatui 模态框
- Gateway：企微回复「请回复 Y 确认执行：...」

---

## 5. 数据模型

### 5.1 配置

路径：`~/.hi/config.toml`

```toml
working_directory = "/path/to/project"

[ai]
provider = "openai-compat"
model = "deepseek-chat"
base_url = "https://api.deepseek.com/v1"
api_key_env = "HI_API_KEY"

[wecom]
bot_id = "your-bot-id"
secret_env = "WECOM_SECRET"
websocket_url = "wss://openws.work.weixin.qq.com"
dm_policy = "open"   # 或 allowlist
allow_from = []      # dm_policy = allowlist 时填写 userid
```

### 5.2 SQLite Schema（MVP）

```sql
-- 用户（v1 一行，v2 扩展）
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    wecom_user_id TEXT UNIQUE,
    display_name TEXT,
    created_at INTEGER NOT NULL
);

-- 会话（按 user_id，非按渠道）
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    working_directory TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 消息
CREATE TABLE messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,          -- system|user|assistant|tool
    content TEXT NOT NULL,
    tool_name TEXT,
    created_at INTEGER NOT NULL
);

-- 渠道 → 用户映射（v2 多人配对）
CREATE TABLE channel_identities (
    channel TEXT NOT NULL,       -- wecom|tui
    external_id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    PRIMARY KEY (channel, external_id)
);
```

### 5.3 会话隔离（当前实现）


| 入口            | SessionId        |
| ------------- | ---------------- |
| `hi tui`      | `tui:main`       |
| `hi chat`     | `chat:main`      |
| 企微用户 `userid` | `wecom:{userid}` |


TUI 与 Gateway 可并发写 SQLite（WAL + 短事务），但 **不共享** 同一会话历史。

---

## 6. 企业微信对接（首个渠道）

### 6.1 连接方式

- 企微管理后台 → 智能机器人 → **API 模式 → 长连接**
- `hi gateway` 主动连接 `wss://openws.work.weixin.qq.com`
- **无需** 公网域名、ngrok/frp、Token、EncodingAESKey

### 6.2 消息流

1. 订阅：`aibot_subscribe`（`bot_id` + `secret`）
2. 入站：`aibot_msg_callback` → 解析 `userid` / 文本 → `wecom:{userid}` 会话
3. 出站：`aibot_respond_msg` 回复；长任务可先回「思考中…」；超长回复按平台上限分多条发送

### 6.3 安全

- `secret` 仅环境变量，不入库
- `dm_policy = allowlist` 时校验 `allow_from`
- v2：可选配对码 / 更细粒度审批（Gateway 侧）

---

## 7. LLM Provider 层

### 7.1 Trait

```rust
#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, request: AiRequest) -> Result<AiResponse>;
    // v2: async fn stream(...)
}
```

### 7.2 实现优先级

1. **OpenAI-compat** — 覆盖最多国内 API
2. **Anthropic Messages API**
3. **Ollama**（本地）

### 7.3 配置切换

`hi config` 向导或编辑 `config.toml`；运行时 `/model`（TUI slash command，v1.1）。

---

## 8. TUI 设计要点


| 区域  | 内容                                    |
| --- | ------------------------------------- |
| 消息区 | user / assistant / tool 输出，可折叠 tool 块 |
| 输入区 | 多行编辑，Enter 发送，Shift+Enter 换行          |
| 状态栏 | cwd、model、token 用量、session id         |
| 快捷键 | Ctrl+C 中断，Ctrl+L 清屏（可选）               |


技术栈：`ratatui` + `crossterm` + `tokio`（agent 在 blocking 或 spawn 任务）。

---

## 9. 向方案 C（Daemon）演进

预留接口：

```rust
// 未来 protocol/
pub trait AgentClient {
    async fn send_message(&self, user_id: &UserId, text: &str) -> Result<()>;
    fn subscribe(&self) -> impl Stream<Item = AgentEvent>;
}
```

- v1：`InProcessAgentClient` — TUI/Gateway 直接持有 `AgentLoop`
- v2：`RpcAgentClient` — Unix socket / HTTP 连接 `hi daemon`

---

## 10. 实现里程碑


| 阶段       | 交付物                                            |
| -------- | ---------------------------------------------- |
| **M0** ✅ | Cargo Workspace 骨架、`hi tui/gateway/config` 占位  |
| **M1** ✅ | `OpenAiCompatProvider` + 最小 AgentLoop（无工具，纯对话） |
| **M2** ✅ | 四工具 + bash 审批 + TUI 基础交互                       |
| **M3** ✅ | SQLite 会话持久化 + 跨进程读写                           |
| **M4** ✅ | 企微 WebSocket + 发消息 + `dm_policy`               |
| **M5** ✅ | 上下文压缩 + 多 Provider                             |
| **M6** ✅ | `hi config` 向导、文档                              |
| **M7** ✅ | 记忆体系：结绳长期记忆、`memory_search` / `memory_write`、会话 append-only（见 [m7-memory-system.md](../exec-plans/active/m7-memory-system.md)） |


---

## 11. 技术选型汇总


| 组件   | 选型                                   |
| ---- | ------------------------------------ |
| 语言   | Rust 2021                            |
| 异步   | tokio                                |
| CLI  | clap                                 |
| TUI  | ratatui + crossterm                  |
| HTTP | reqwest（LLM API）；Gateway 走 WebSocket |
| 存储   | rusqlite（WAL）                        |
| 序列化  | serde + toml + serde_json            |
| 日志   | tracing                              |


---

## 12. 风险与缓解


| 风险                       | 缓解                                   |
| ------------------------ | ------------------------------------ |
| Rust 学习曲线陡               | 分里程碑；M0–M2 先跑通主路径                    |
| 企微长连接鉴权失败                | 核对 `bot_id`、`WECOM_SECRET`、后台是否启用长连接 |
| TUI + Gateway 并发写 SQLite | WAL + 短事务；极端情况 session 级 mutex       |
| LLM tool calling 格式差异    | OpenAI-compat 先行；Anthropic 单独适配      |
| 个人微信需求                   | 明确非 MVP；v3 评估桥接方案                    |


---

## 附录 A：仓库结构（当前）

```
hi/
├── Cargo.toml
├── README.md
├── app/              # package: hi, binary: hi
├── core/             # package: hi-core (tools/, store/ 为内部 mod)
├── ai/               # package: hi-ai
├── tui/              # package: hi-tui
├── gateway/          # package: hi-gateway
└── docs/
    └── design/
        └── 2026-05-22-hi-agent-design.md
```

## 附录 B：MVP 能力清单


| 能力              | hi MVP                             |
| --------------- | ---------------------------------- |
| Workspace crate | 5（app / core / ai / tui / gateway） |
| 默认 4 工具         | ✅（hi-core::tools）                  |
| TUI             | hi-tui                             |
| Gateway         | hi-gateway（企微 WebSocket；更多渠道待接）    |
| Skills / MCP    | ❌ 后续                               |
| 会话              | 按渠道隔离 `session_id`                 |


---

## 演进说明（2026-05-22 后）

与初稿相比，实现已简化：

### 企微接入

- **不再使用**：自建应用 HTTP 回调 + Token/EncodingAESKey + ngrok/frp
- **改为**：智能机器人 WebSocket 长连接（`wss://openws.work.weixin.qq.com`）
- 配置：`[channels.wecom]` 的 `bot_id` + `secret`（写入 `~/.hi/hi.toml`）；可选 `dm_policy` / `allow_from`（旧版 `[wecom]` 仍可加载）

### 统一配置（2026-06 后）

- 配置文件：`~/.hi/hi.toml`（合并原 `config.toml` + `channels.toml`）
- LLM：`[ai].default` + `[ai.providers.<name>]`
- 渠道：`[channels.wecom]` / `[channels.feishu]` / `[channels.weixin]`
- 向导：`hi setup` + `hi gateway setup`

### 会话模型

- `hi tui` → `tui:main`
- `hi chat` → `chat:main`
- 企微用户 → `wecom:{userid}`（每人独立）

