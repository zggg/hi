# ExecPlans 概览

复杂功能或跨 crate 重构使用 [exec-plans/](exec-plans/) 目录管理实施计划。

## 目录

```
docs/exec-plans/
├── active/           进行中的计划
├── completed/        已完成计划（保留决策上下文）
└── tech-debt-tracker.md
```

## 何时写 ExecPlan

- 涉及多个 crate 的功能（如 M2 四工具 + TUI）
- 架构变更（如引入 `hi daemon`）
- 预计数小时以上的任务

## 何时不写

- 单文件 bug 修复
- 文档更新
- 配置微调

详见 [exec-plan 标准](.cursor/skills/harness-init/references/exec-plan-template.md)（skill 参考）。
