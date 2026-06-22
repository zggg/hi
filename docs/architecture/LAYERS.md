# 分层架构（权威）

本文档是 hi 项目 **crate 间依赖** 的单一事实来源。边界测试与一致性脚本均引用此文件。

## 层级定义

| 层级 | Crate | 目录 | 职责 |
|------|-------|------|------|
| **entry** | `hi` | `app/` | CLI 解析、配置加载、组装各 crate |
| **adapters** | `hi-tui` | `tui/` | 终端 UI，订阅 `AgentEvent` |
| **adapters** | `hi-gateway` | `gateway/` | 消息渠道网关、`ChannelAdapter` |
| **foundation** | `hi-core` | `core/` | Agent 运行时、工具、会话存储、配置 |
| **foundation** | `hi-ai` | `ai/` | LLM Provider 抽象与实现 |

## 依赖规则

```
entry      → adapters, foundation（全部允许的 foundation crate）
adapters   → foundation（仅 hi-core）
foundation → 无 hi 内部 crate 依赖
```

### 允许依赖矩阵

| 来源 \ 目标 | hi-core | hi-ai | hi-tui | hi-gateway | hi |
|-------------|---------|-------|--------|------------|-----|
| hi-core     | —       | ❌    | ❌     | ❌         | ❌  |
| hi-ai       | ❌      | —     | ❌     | ❌         | ❌  |
| hi-tui      | ✅      | ❌    | —      | ❌         | ❌  |
| hi-gateway  | ✅      | ❌    | ❌     | —          | ❌  |
| hi (app)    | ✅      | ✅    | ✅     | ✅         | —   |

### hi-core 内部模块（同 crate，非 crate 边界）

`core/src/` 内模块无跨 crate 限制，但应遵循：

- `tools/` — 文件与 shell 工具实现
- `store/` — SQLite 持久化
- `config`, `session`, `event` — 运行时类型

## 修复指引

当出现边界违规时，按以下步骤修复：

1. **确认违规类型**
   - Cargo.toml 中 `[dependencies]` 出现不允许的 `hi-*` crate
   - 源码中 `use hi_*` 来自不允许的层

2. **常见场景**

   | 违规 | 修复 |
   |------|------|
   | `hi-core` 依赖 `hi-ai` | 将 Provider 注入移到 `hi (app)` 组装层，通过 trait 对象传入 `AgentLoop` |
   | `hi-tui` 依赖 `hi-gateway` | 提取共享逻辑到 `hi-core`，TUI 与 Gateway 各自调用 core API |
   | `hi-gateway` 依赖 `hi-tui` | 同上 —— 适配器层不得互相依赖 |
   | `hi-ai` 依赖 `hi-core` | AI 层保持独立；core 定义的消息类型通过 app 层转换 |

3. **验证**

   ```sh
   cargo test -p architecture-tests
   ```

4. **若暂时无法修复（已有仓库）**
   - 将违规条目加入 `architecture-tests/known-violations.json`
   - 条目只能删除（修复后），不能新增

## 错误信息格式

```
VIOLATION: {file}:{line} imports {target} — {from_layer} cannot import {to_layer}. See docs/architecture/LAYERS.md
```

## 演进预留

- v2 `hi daemon`：`hi-tui` / `hi-gateway` 通过 RPC 客户端连接 daemon，分层规则不变
- `hi-ai` 新增 Provider 实现：仍在 `ai/` crate 内，不引入向上依赖
