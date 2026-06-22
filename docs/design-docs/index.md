# 架构决策记录（ADR）索引

| ID | 标题 | 状态 | 文件 |
|----|------|------|------|
| — | 初始设计（2026-05-22） | accepted | [../design/2026-05-22-hi-agent-design.md](../design/2026-05-22-hi-agent-design.md) |
| — | 结绳记事长期记忆（2026-06-04） | proposed | [../design/2026-06-04-knot-memory-design.md](../design/2026-06-04-knot-memory-design.md) |
| 0001 | 记忆抽取触发策略 | accepted | [0001-memory-extraction-triggers.md](0001-memory-extraction-triggers.md) |
| — | Harness 脚手架初始化 | accepted | AGENTS.md + docs/architecture/LAYERS.md |

## 如何新增 ADR

1. 在 `docs/design-docs/` 创建 `{NNNN-short-title}.md`
2. 更新本索引
3. 若违背 [core-beliefs.md](core-beliefs.md)，必须先讨论并修订 core-beliefs

## ADR 模板

```markdown
# {Title}

## Status
proposed | accepted | deprecated

## Context
...

## Decision
...

## Consequences
...
```
