# M12：SQLite 并发访问（1 写 + 读连接池）

> 状态：**已完成**（Phase 0–3）

This ExecPlan is a living document.

## Purpose

M3 已启用 WAL，支持 **跨进程** 共享 `~/.hi/data/sessions.db`。M12 在 **foundation 层** `SessionStore` 实现 **1 条写连接 + 只读连接池**，提升进程内读并发；**所有 I/O 适配器经 `HiServices` 共享同一实例**，adapter 层零改动。

配置：`[storage].read_pool_size`（默认 4，范围 2–8），`hi setup` 写入 `hi.toml`。

## 已锁定的设计决定

| # | 决定 | 取值 |
|---|------|------|
| 1 | 形态 | **1 条写连接** + **只读连接池**（不做写侧池化——SQLite 单写者，写池无收益） |
| 2 | 读池大小 | 默认 **4**（`hi.toml` 或常量可配，范围 2–8） |
| 3 | 实施路径 | **直接上池**，跳过「单读 + 单写」中间态 |
| 4 | 改动范围 | 仅 `hi-core::SessionStore` 及 store 子模块 |
| 5 | turn 内读 context | 仍走 **写连接**（与 append 一致；避免 WAL 读连接略滞后） |
| 6 | 旁路只读 | `list_sessions` / `load_*` / `list_knots` 等 → **读池** |

## 在架构中的位置

```
hi-core::SessionStore          ← 本计划改这里（foundation）
  write: Mutex<Connection>     ← 所有写 + turn 内读 context
  read_pool: ReadPool          ← 旁路只读（N 条只读连接）
       ↑
HiServices（Arc<SessionStore>，单例）
       ↑
TUI / chat / gateway / hi session CLI /（未来 M11 HTTP）
```

**共享同一 `SessionStore` 的典型路径**

| 消费者 | 读 | 写 |
|--------|----|----|
| `AgentLoop::with_persistence` | 加载 context → **write** | append 消息 → **write** |
| `hi session list/show` | **read_pool** | — |
| gateway 各渠道 | 经 `run_turn` → **write** | 同上 |
| 记忆（knots） | `list_*` → **read_pool** | 写入 → **write** |

## 现状（已实现）

```rust
// core/src/store/mod.rs
pub struct SessionStore {
    write: Mutex<Connection>,
    read_pool: ReadPool,
    path: String,
}
```

- **写连接**：turn 内读 context、append、压缩、knot 写、purge。
- **读池**：旁路只读 API；大小 `[storage].read_pool_size`（默认 4）。
- WAL（M3）跨进程；`busy_timeout` 5s。

## 实现结构

```rust
/// 只读连接池：借出 → 用毕归还；WAL 下多 reader 与 writer 并行。
struct ReadPool {
    // 实现二选一（见下「池实现」）
}

pub struct SessionStore {
    write: Mutex<Connection>,
    read_pool: ReadPool,
    path: String,
}
```

### 连接路由规则

| API 类别 | 连接 | 示例 |
|----------|------|------|
| **写** | `write` | `append_message`, `purge_session`, knot insert/update, schema migrate |
| **turn 内读** | `write` | `load_context_messages`（AgentLoop 回合路径） |
| **旁路读** | `read_pool` | `list_sessions`, `load_all_messages`, `load_messages_range`, `list_compressions`, `get_compression`, `list_knots`, `list_all_knots`, `get` knot |

> 若某 `load_*` 同时被 turn 与 CLI 调用，turn 路径走 write，CLI/HTTP 走 read_pool。必要时在 `SessionStore` 方法层显式拆分（如 `load_context_messages` vs `load_all_messages_for_display`），或内部参数 `for_agent_turn: bool`——实现时选最小 diff 方案。

### 池实现

**首选（推荐）**：自管小池，**零新依赖**

- `open()` 时用 `SQLITE_OPEN_READ_ONLY` 打开 `pool_size` 条连接；
- `ReadPool` 内 `Mutex<Vec<Connection>>` 或 `crossbeam` 无锁栈（可选）做借还；
- `get()` 阻塞直到有空闲连接（或超时返回 `Error`）。

**备选**：`r2d2` + `r2d2_sqlite`——仅在自管借还/超时逻辑变复杂时再引入。

### open() 流程

1. 打开 **write** 连接 → `ensure_compatible` + WAL pragma（与 M3 一致）。
2. 打开 **pool_size** 条 read-only 连接 → 同一 path；每条执行 schema 兼容性检查（只读连接可 `query_only` + 读 `user_version`）。
3. 失败时关闭已打开连接，返回错误。

### 配置（可选，M12 可先用常量）

```toml
# hi.toml — 后续可加；M12 Phase 1 可用常量 READ_POOL_SIZE = 4
[storage]
read_pool_size = 4
```

## 预期收益（诚实预期）

| 场景 | 是否变快 |
|------|----------|
| 多 session 并发 `list` / `load` 历史 + gateway 写 transcript | **是**（读并行、读写不互堵） |
| 单人单 session TUI / chat | 几乎无感 |
| 单次 turn（读 context → LLM → append） | 有限（仍走 write + coordinator 串行） |
| 写吞吐 | **否**（SQLite 仍单写者） |

## 与其它里程碑的关系

| 里程碑 | 关系 |
|--------|------|
| **M3** | M12 是 M3 的进程内并发演进，不替代 WAL / append-only |
| **M11 HTTP** | 众多消费者之一；M12 **优先完成**；M11 不阻塞于 M12 |
| **gateway** | turn 限流在 adapter；DB 在 core——两层独立 |

## 循环目标（每 Phase 结束必跑）

```sh
cargo test -p hi-core -- store
cargo test --workspace
cargo clippy --workspace -- -D warnings
./scripts/check-consistency.sh
```

| Phase | 交付 | 验收 |
|-------|------|------|
| **0** | `ReadPool` + 新 `SessionStore` 结构；`open()` 双路径建连 | 编译通过；现有 store 单测仍绿（或暂保留旧 open 并行，Phase 1 切换） |
| **1** | 所有 store API 按「连接路由规则」分流；turn 内 load 走 write | `cargo test -p hi-core -- store` 全绿 |
| **2** | 并发测试：≥4 线程同时 `list_sessions` / `load_all` + 1 线程 `append` | 无 panic、无 SQLITE_BUSY 泄漏；测试在 CI 稳定 |
| **3** | M3 / AGENTS / `core/src/store/mod.rs` 模块注释更新 | check-consistency |

## Key Files

```
core/src/store/mod.rs          # SessionStore、ReadPool、open、conn 分流
core/src/store/read_pool.rs    # 新建（若 mod 过大）
core/src/store/sessions.rs
core/src/store/messages.rs
core/src/store/knots.rs
core/src/store/compressions.rs
core/test/unit/store/          # 并发读 + 读写交错测试
core/Cargo.toml                # 仅当选用 r2d2 时加依赖
docs/exec-plans/active/m3-sqlite-sessions.md  # 交叉引用（可选）
```

> **刻意不改**：`app/`、`gateway/`、`tui/`——仍 `SessionStore::open` + `Arc` 经 `HiServices` 注入。

## Progress

- [x] Phase 0 — ReadPool + SessionStore 结构 + open
- [x] Phase 1 — API 分流（write / read_pool）
- [x] Phase 2 — 并发集成测试
- [x] Phase 3 — 文档（M3 / setup / ARCHITECTURE / store 模块注释）

## 风险

- **WAL + 只读连接**：确认 `SQLITE_OPEN_READ_ONLY` 下能读 WAL 未 checkpoint 的最新页；turn 内读仍走 write 规避一致性问题。
- **SQLITE_BUSY**：写连接与多 read 并发时偶发；写路径保持短事务；必要时 `busy_timeout` pragma（如 5s）。
- **schema migrate**：仅 **write** 连接执行 migrate；migrate 后 read 池连接需 **重建** 或 `open()` 在 migrate 之后创建 read pool。
- **池耗尽**：读池满时阻塞或快速失败；日志 + 单测覆盖。
