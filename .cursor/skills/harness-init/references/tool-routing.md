# 按平台进行工具路由

Harness-init 按**意图**委派工作，而不是按具体工具名委派。在 Cursor 环境中，skill 默认以内联方式完成这些意图。

## 意图 → Cursor Skill 映射

| Intent | Model Tier | Cursor |
|--------|-----------|--------|
| **Explore** | Lightweight | Inline（使用 `@codebase`、搜索、目录阅读） |
| **Architect** | Heavyweight | Inline（在对话中做架构分析） |
| **Write** | Lightweight | Inline（直接生成或修改文件） |
| **Execute** | Standard | Inline（直接实现、补配置、改脚本） |
| **Verify** | Standard | Inline（运行检查、核对结果） |

## 模型层级

| Tier | 用途 | 示例 |
|------|---------|----------|
| Lightweight | 快速、低成本任务（列文件、生成文档） | Haiku、GPT-4o-mini |
| Standard | 实施与验证 | Sonnet、GPT-4o |
| Heavyweight | 架构决策、深度分析 | Opus、o3 |

## 兜底方式

在 Cursor 中没有明确的多代理委派层时：
- 全部意图都在主对话里顺序完成
- 把模型层级当作“哪些任务需要更多推理预算”的提示
- 这个 skill 仍然可以使用，只是以内联方式执行
