# 长期记忆（结绳）使用指南

> hi 的长期记忆称为「结绳记事」（knot）。本指南讲**如何用命令查看/管理记忆**，以及 `memory list` 各列的含义。
> 定义源码：`app/src/memory.rs`（CLI）、`core/src/memory/`（核心逻辑）。
> 最后更新：2026-06-08

---

## 1. 命令一览

```sh
# 开发态用 cargo，或用已安装的 hi 二进制（下同）
cargo run -p hi -- memory list                # = hi memory list
```

| 命令 | 作用 |
|------|------|
| `hi memory list` | 列出当前 owner 的**活跃**记忆 |
| `hi memory list --all` | 连同非活跃（superseded / deleted）一并列出 |
| `hi memory list --owner <id>` | 指定 owner（默认取 `[memory].owner_id`，本机为 `local`） |
| `hi memory show <id>` | 查看单条全部字段（owner/kind/status/confidence/clarity/permanent/hash/时间） |
| `hi memory add "<内容>" --kind <kind> [--confirmed] [--permanent] [--owner <id>]` | 手动新增一条 |
| `hi memory forget <id>` | 软删除（status = deleted） |
| `hi memory reinforce <id> [--permanent]` | 强化（提升 clarity / 置为永久不衰减） |
| `hi memory extract --session <id> [--owner <id>]` | 用 LLM 从指定会话 transcript 抽取记忆 |

> 记忆功能需 `[memory].enabled = true`（hi.toml）。关闭时 `extract` 会直接报错提示去启用。
> Agent 在对话中也可用 `memory_search` 工具按需检索（待办 / 决定等）；system 仅注入偏好 + 事实基线。

---

## 1.5 记忆何时**自动**新增（非手动）

除上面的 `memory add` / `memory extract` 手动命令外，对话过程中只有三条自动写入路径，均需 `[memory].enabled = true`：

| 触发 | 配置 | 说明 |
|------|------|------|
| 回合结束后（信号触发） | `extract_after_turn = true` + `extract_after_turn_cue_only = true`（默认） | **无状态单回合判定**：仅当本回合命中记忆信号词（记住 / 我叫 / 我喜欢 / remember …）或新内容达到 `extract_turn_min_tokens`（默认 200）时才抽结。把 `extract_after_turn_cue_only` 设为 `false` 可退回旧的「每轮都抽」。 |
| 上下文压缩时 | `extract_on_compress = true`（默认） | 发生 LLM 摘要式压缩时，对被裁剪掉的历史段抽结（紧急裁剪不触发）。 |
| Agent 主动记 | `memory_write_tool = true`（默认） | 模型判断有值得长期记住的偏好 / 事实 / 决定 / 待办时，调用 `memory_write` 工具即时写入（自动去重）。 |

> 设计动机与权衡见 `docs/design/2026-06-04-knot-memory-design.md` §7.2。彻底关闭自动新增（仅保留手动）：把 `extract_after_turn`、`extract_on_compress`、`memory_write_tool` 都设为 `false`。

---

## 2. `memory list` 列含义

输出表头：`ID  KIND  CONF  CLR  CONTENT`（格式源码：`app/src/memory.rs` `print_knot_header` / `print_knot_line`）。

### ID
记忆主键，用于 `show` / `forget` / `reinforce`。

### KIND — 记忆类型（`KnotKind`）

| 值 | 含义 |
|------|------|
| `preference` | 偏好（如「用简体中文」） |
| `fact` | 事实（如「在用 macOS」） |
| `decision` | 决定 |
| `task` | 待办（带 `[ ]` 未完成 / `[x]` 已完成 状态） |
| `procedure` | 流程 / 操作步骤 |

### CONF — 置信度（`KnotConfidence`）

记忆的可信来源，决定**初始 clarity** 与是否**默认永久**：

| 值 | 含义 | 初始 CLR | 默认永久 |
|------|------|:-------:|:-------:|
| `confirmed` | 用户明确说过 / 确认（`--confirmed`） | 1.0 | 是 |
| `inferred` | LLM 从对话**推断**得出 | 0.7 | 否 |
| `dream` | 最弱的「臆测 / 未验证」级，注入门槛更高 | 0.4 | 否 |

来源：`core/src/memory/merge.rs` `initial_clarity`、`core/src/memory/types.rs`。

### CLR — 清晰度（clarity，0.00–1.00）

这条记忆当前的「鲜活 / 可信强度」，越接近 1 越强（显示两位小数）。两个作用：

1. **会衰减（忘川 decay）**：随时间按半衰期下降（`[memory].decay_half_life_days`，默认 30 天），除非该记忆为 `permanent`。列表展示的是当前值。
   - 源码：`core/src/memory/decay.rs` `effective_clarity` / `decay_clarity`。
2. **决定是否注入系统提示**：低于 `[memory].inject_clarity_threshold`（默认 0.35）的不会注入给模型；`dream` 类还需 ≥ 0.6 才注入。
   - 源码：`core/src/memory/inject.rs`。

### CONTENT
记忆正文（列表中超过 60 字符会截断显示 `…`，完整内容用 `hi memory show <id>`）。

### 状态后缀
非活跃记忆会在行尾标注 `[superseded]` 或 `[deleted]`（`KnotStatus`，`core/src/memory/types.rs`）。`hi memory list` 默认只显示 `active`，加 `--all` 才会看到其它状态。

---

## 3. 示例

```text
    ID  KIND         CONF       CLR  CONTENT
     1  preference   confirmed  1.00  偏好简体中文
     2  fact         inferred   0.63  使用 macOS，常用 DeepSeek 与 Codex 两个 Provider
     3  task         inferred   0.70  [ ] 给 codex 运行时补充工具越界回退提示
```

- 第 1 条：用户确认过的偏好，clarity 满格、永久。
- 第 2 条：从对话推断的事实，clarity 已随时间衰减到 0.63。
- 第 3 条：待办，`[ ]` 表示未完成。
