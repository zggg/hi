# 上下文策略：静态 vs 动态

## 静态上下文（存在仓库里，始终可用）

| Artifact | File | 作用 |
|----------|------|------|
| 导航地图 | `AGENTS.md` | Agent 入口，约 100 行索引 |
| 分层规则 | `docs/architecture/LAYERS.md` | 权威依赖层级 |
| 典范模式 | `docs/golden-principles/*.md` | DO/DON'T，30-60 行 |
| 开发指南 | `docs/guides/*.md` | setup、testing、deployment |
| ExecPlan 标准 | `docs/exec-plans/` 或 `PLANS.md` | 复杂功能实施模板 |
| 约束 | Linter rules + boundary tests | 机械强制，而不是 prose |

## 动态上下文（每次会话开始时探测）

| Signal | Source | 作用 |
|--------|--------|------|
| 工作进度 | `git status` + `git log --oneline -10` | 当前做到哪、停在哪里 |
| 代码健康度 | LSP diagnostics / linter output | 先修什么问题 |
| 未完成任务 | 会话状态目录或项目任务追踪器 | 从哪里继续 |
| 架构合规性 | 运行 boundary test（若存在） | 是否出现新的越层导入 |
| 文档漂移 | 比较 docs/ 与 src/ 的更新时间 | 文档是否过期 |
| 应用可观测性 | 应用日志、指标、追踪（若可用） | 运行时错误与性能问题 |

## 两者区别

- **Static** = “规则是什么” —— 答案在不同会话间不变
- **Dynamic** = “当前状态如何” —— 每次都必须重新探测

两者缺一不可。Static 提供地图，Dynamic 提供地形。

## 优雅降级

并不是所有动态信号都一定可用。缺失时这样处理：

| Signal | 如果不可用 | 兜底 |
|--------|---------------|----------|
| LSP diagnostics | 没有运行中的 LSP 服务 | 改为运行 linter CLI |
| Session state | 没有 `.omc/state/` | 跳过，并标注为全新会话 |
| Boundary test | 还没创建 | 标注：将在 Phase 3 创建 |
| App observability | 无法访问日志/指标 | 跳过，并注明不可用 |
