# 记忆抽取触发策略（结绳抽结的时机）

## Status

accepted（2026-06-08 实施；取代「每轮无条件抽取」）

> 关联：[结绳记事长期记忆](../design/2026-06-04-knot-memory-design.md) §7.2、[memory-usage 指南](../guides/memory-usage.md) §1.5

## Context

结绳记事（knot memory）首版的自动抽结有两条路径：

1. `extract_after_turn`：每个回合 `TurnCompleted` 后，对本回合 transcript 调一次 LLM 抽结；
2. `extract_on_compress`：上下文 LLM 摘要式压缩时，对被裁剪段抽结。

第 1 条采用**最细粒度的节奏触发**——「用户说一句 → Agent 答完」就抽一次。实际使用中暴露出问题（不是「记忆爆炸」，已有 `content_hash` 去重 + clarity 衰减 + dream 不注入能部分自愈，而是**触发依据错了**）：

| 维度 | 每轮抽结的代价 |
|------|----------------|
| 成本 | 每回合 +1 次 LLM 调用，且每次都带「已有结（最多 50 条）+ 本轮 transcript」，记忆越多 prompt 越贵，近乎把会话调用量翻倍 |
| 信噪比 | 「你好 / 谢谢 / 1+1=?」这类轮次绝大多数无可记之物，仍照常付费抽一次 |
| 语义割裂 | 偏好 / 决定常跨多轮才成形，单轮抽结只能抓碎片，更易产生措辞略异、hash 不撞的近重复结 |
| 场景适配 | hi 是轻量助手，要跑 DeepSeek / 本地 Ollama，还走企微渠道，每条消息的成本与延迟都敏感 |

一句话：它在「**既无证据、也无必要**」的时刻做了昂贵的事。

### 关键运行时约束

持久化模式（Gateway，`PersistedAgentHost`）下**每个回合都会重建 `AgentLoop`**（`app/src/services.rs` 每次 `run_turn` 都 `build_agent_loop`）；TUI 则复用同一个长生命周期 `AgentLoop`。

**推论**：任何「跨回合内存计数器」（如「攒够 N 轮再批量抽」）在 Gateway 下每回合清零、攒不起来——会变成**看似可配、实则失效**的配置。要做跨回合批量必须落到 DB 持久化水位。

## Decision

### 好的触发只应基于三类依据

| 依据 | 含义 | 落地 |
|------|------|------|
| 证据（evidence） | 本回合出现明确「值得记」的信号 | 显式信号门（极便宜、高精度） |
| 必要（necessity） | 原始消息即将离开工作集 | `extract_on_compress`（保留） |
| 摊销（amortization） | 累计足够多未抽取内容 | 由压缩抽结承担，不引入跨回合计数 |

### 最终方案：无状态单回合判定 + 必要性兜底 + Agent 主动写

`extract_after_turn` 不再无条件触发，改为**无状态单回合判定**（跨 TUI / Gateway 行为一致，不依赖任何跨回合状态）：

- **信号门**：本回合用户消息命中记忆信号词（`记住 / 我叫 / 我喜欢 / remember …`）→ 抽。
  - 实现：`core/src/memory/extract.rs` `turn_has_memory_cue`，刻意选高精度词，**避开**「我是 / 我在」这类高频词以防过度触发；隐式偏好交由压缩抽结与 `memory_write` 兜底。
- **体量门**：本回合新内容 token 估算 ≥ `extract_turn_min_tokens`（默认 200）→ 抽，捕获实质性长对话。
- 两门都不命中 → 跳过。
- 逃生开关：`extract_after_turn_cue_only = false` 退回旧的「每轮无条件抽」。

并新增 **`memory_write` 工具**（`core/src/tools/memory_write.rs`）：与 `memory_search` 读路径对称，让模型在回合内判断有值得长期记住的偏好 / 事实 / 决定 / 待办时**主动写入**（走 `merge_knot` 自动去重），**零额外 LLM 调用**。默认 `memory_write_tool = true`。

判定逻辑见 `core/src/agent.rs` `extract_after_turn`。

### 被否决的备选方案

1. **删掉所有自动抽结，纯靠 `memory_write` 工具**（最「现代」、零额外调用、精度最高）。
   - 否决：① 依赖模型自觉调用，对 DeepSeek / 本地 Ollama 等非顶级 Provider 不稳定；② 短会话里重要事实若从未压缩、模型又没调工具 → 永久丢失。**不能作默认**，仅作能力强模型的增强路径。
2. **跨回合「攒够 N 轮 / T tokens」批量抽**。
   - 否决：Gateway 每回合重建 `AgentLoop`，内存计数器攒不起来；做成 DB 持久化水位则复杂度与收益不匹配，而「必要性」已由压缩抽结覆盖。

## Consequences

- 默认配置下，无意义轮次不再触发抽结 LLM 调用，成本与噪声显著下降；TUI 与 Gateway 行为一致。
- 隐式偏好（未命中信号词、回合不大、未触发压缩）可能延迟到压缩时才被抽——由 `extract_on_compress` 与 `memory_write`（能力模型）兜底，权衡可接受。
- 彻底关闭自动新增（仅保留 `hi memory add` / `hi memory extract` 手动）：将 `extract_after_turn`、`extract_on_compress`、`memory_write_tool` 全设为 `false`。

### 配置（`[memory]`）

| 字段 | 默认 | 作用 |
|------|------|------|
| `extract_after_turn` | `true` | 启用「回合结束后抽取」路径 |
| `extract_after_turn_cue_only` | `true` | 仅信号 / 体量触发；`false` = 每轮无条件抽 |
| `extract_turn_min_tokens` | `200` | 本回合新内容达到该 token 量也触发（0 = 关体量门） |
| `extract_on_compress` | `true` | 压缩时对被裁剪段抽结（必要性兜底） |
| `memory_write_tool` | `true` | 暴露 `memory_write` 工具供 Agent 主动记 |

定义源：`core/src/config/memory.rs`。
