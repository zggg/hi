# ExecPlan 标准（docs/exec-plans/）

有两个互补来源：
- **OpenAI Harness article** —— `docs/exec-plans/` 目录 + 生命周期（active → completed）
- **OpenAI Cookbook PLANS.md** —— 单文件 ExecPlan 格式（自包含的活文档）

适用于复杂功能或较大的重构。小改动不需要。

## 目录结构（来自 Harness article）

```
docs/exec-plans/
├── active/                # 进行中的计划
│   └── {feature-name}.md  # 每个功能/重构一个计划
├── completed/             # 已完成计划及复盘
│   └── {feature-name}.md  # 保留给后续 agent 做上下文
└── tech-debt-tracker.md   # 已知技术债与优先级（可选）
```

完成后的 active 计划应移动到 `completed/`。这样后续 agent 无需人工补充背景也能理解历史决策。

## 单文件替代方案

对于更简单的项目，可以在根目录或 `.agent/PLANS.md` 使用单个 `PLANS.md`。如果项目会并行推进多个功能，更推荐使用目录结构。

## 核心要求

- **完全自包含** —— 新加入的人只看这份文档也能实施
- **活文档** —— 要持续更新进展、意外、决策
- **可恢复** —— 仅凭 ExecPlan 就能从中断点继续

## 必需章节

```markdown
# <Short action-oriented description>

This ExecPlan is a living document.

## Purpose / Big Picture
What the user gets, how to see it working.

## Progress
- [x] (YYYY-MM-DD HH:MMZ) Completed step
- [ ] Pending step

## Surprises & Discoveries
- Observation: ...
  Evidence: ...

## Decision Log
- Decision: ...
  Rationale: ...
  Date/Author: ...

## Outcomes & Retrospective
Summary of results, gaps, and lessons learned.

## Context and Orientation
Current state, assume reader knows nothing.

## Plan of Work
Edit and addition sequence, name specific files.

## Concrete Steps
Exact commands with working directory, include expected output.

## Validation and Acceptance
How to start/use the system. Observable behavior, not internal properties.

## Idempotence and Recovery
Are steps repeatable? Rollback path.

## Artifacts and Notes
Key output examples, keep concise.

## Interfaces and Dependencies
Libraries, modules, function signatures to use.
```

## 何时使用

- 涉及多个文件/模块的复杂功能
- 影响架构的大型重构
- 需要数小时以上的实施任务

## 何时不要使用

- Bug 修复
- 小功能补充
- 单纯配置改动
