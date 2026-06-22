---
name: harness-init
description: 用于初始化新项目、让仓库具备 agent-ready 能力，或添加架构分层边界 —— 会生成 AGENTS.md、docs/ 单一事实来源、边界测试、linter 规则和轻量一致性检查
triggers:
  - harness-init
  - harness engineering
  - architecture boundaries
  - layer enforcement
  - init harness
  - 项目初始化
  - 初始化脚手架
  - 初始化仓库规范
  - 建立项目文档体系
  - 补项目规范
  - 补仓库文档
  - 整理老项目结构
  - 让 AI 能接手维护
  - 设置架构边界
  - 设置分层约束
  - 建立代码边界
argument-hint: "[full|N|N-M]"
metadata:
  author: Gizele1
  version: "1.1.0"
---

# Harness Init

<Purpose>
用 OpenAI 的 harness engineering 脚手架初始化仓库：生成 AGENTS.md 导航地图、docs/ 单一事实来源、架构边界约束、golden principles，以及静态/动态上下文策略。

这是 harness engineering 的**仓库初始化子集**。运行时反馈回路、agent review 回路和可观测性栈搭建不在本 skill 范围内。如果仓库已经具备可观测性（日志、指标、追踪），本 skill 会把它作为动态上下文读取，但不会主动搭建。

来源：OpenAI《Harness engineering: leveraging Codex in an agent-first world》（2026-02-11）
</Purpose>

<Why_This_Exists>
AI agent 只能处理它们能看见的内容。没有结构化文档、机械化约束和明确的分层边界，agent 就会做出不一致的架构决策、引入越层导入，并产出偏离团队约定的代码。这个 skill 的目标是前置环境设计，让后续每次 agent 会话都从一张地图开始，而不是从空白开始。
</Why_This_Exists>

<Use_When>
- 用户显式输入 `/harness-init`
- 用户显式输入 `/harness-init 0-5`、`/harness-init 0-2`、`/harness-init 3-5`
- 用户明确要求“按 `/harness-init` 的默认流程执行”
- 用户说出与仓库规范、文档体系、分层边界相关的典型关键词或需求表达
</Use_When>

<Do_Not_Use_When>
- 仓库已经有 AGENTS.md + docs/architecture/LAYERS.md + boundary tests —— 直接使用现有 harness
- 用户想要按目录层级拆分的 AGENTS.md —— 这应交给 per-directory init 工具
- 用户想搭建运行时可观测性或 agent review 回路 —— 不在本 skill 范围内
- 只是修一个小 bug 或做一个小功能 —— 直接做事即可
- 用户只是随口提到“整理一下”“补文档”“加规范”，但上下文并不指向仓库级规范化工作
- 用户想头脑风暴或探索方案 —— 这个 skill 用于结构化脚手架，不用于讨论
</Do_Not_Use_When>

<Principles>
1. **给 agent 一张地图，而不是百科全书** —— AGENTS.md 控制在 ~100 行，采用渐进式披露
2. **如果 agent 看不到，它就等于不存在** —— 所有知识都要以机器可读形式落在仓库里
3. **用机械方式强制架构** —— 依赖 linters 和 tests，而不是 markdown 说明
4. **每条错误信息都是 agent 上下文** —— 错误输出里要有修复指引
5. **无聊技术赢** —— 选择可组合、稳定、训练充分的 API
6. **架构测试是棘轮** —— `KNOWN_VIOLATIONS` 只能减少，不能增加
7. **一致性检查服务于仓库，不服务于平台** —— 只检查目标仓库的结构、文档和边界漂移，不绑定 CI
</Principles>

<Execution_Policy>
- Phase 0（Discovery）是强制步骤 —— 绝不能跳过，绝不能假设技术栈
- **参数解析：** `full` = 全部阶段。`N` = 单个阶段。`N-M` = 阶段范围。不传参数 = 交互式（询问用户要搭哪些部分）。无论如何都会先执行 Phase 0。
- 先读再写 —— 匹配现有代码风格和模式
- 对 Java / Spring Boot 项目，**标准多模块结构**（`common/domain/application/infrastructure/interfaces/bootstrap`）只作为条件模式使用，不作为默认迁移目标
- 仅在以下情况启用标准多模块结构：
  1. 用户显式要求按该结构初始化
  2. 仓库基本为空，需要先生成项目骨架
  3. 现有仓库已经明显接近该结构（已存在上述模块中的大部分）
- 若现有老项目结构明显不是该模式（单模块、传统分层、按业务域分包等），禁止强行迁移到该结构；先按真实现状建文档和边界
- 使用 `git mv` 做文档重组，保留历史
- 若仓库已有违规，新增 lint 规则应先 warn，再逐步收敛，不能直接把构建打爆
- 能并行的 phase 尽量并行（例如文档生成与结构扫描）
- 长耗时操作（npm install、全量测试）放后台执行
- **阶段检查点：** 每个 phase 完成后都要验证产物存在（文件已生成 + 相关测试/lint 通过）。记录已完成阶段，便于中断后恢复。
- **失败处理：** 某个 phase 失败时，跳过该 phase，报告失败内容和原因，然后继续执行后续独立 phase。不要因为单个 phase 失败而中止整个流程。
- **验证证据：** “Phase 完成” = 产物文件存在，且相关测试/lint 通过。仅文件存在不算完成。
</Execution_Policy>

<Steps>
1. **Phase 0 — Discovery**（绝不能跳过）
   a. 检测技术栈：语言、框架、包管理器、构建工具、测试框架、linter
   b. 绘制目录结构（`maxdepth 3`，排除 `node_modules/.git`）
   c. 检查已有文档：AGENTS.md、CURSOR.md、docs/、tests、lint config
   d. 通过实际 import 模式识别架构层 —— `Read references/layer-templates.md` 获取常见模型
   e. 注入动态上下文：git status、diagnostics、架构边界测试状态 —— `Read references/context-strategy.md` 获取完整信号表
   f. 命令归一化规则：如果是 Gradle 项目，默认统一使用本地 `gradle` 命令，不使用 `./gradlew`
   g. 对 Java / Spring Boot 项目，先判断是否属于“标准多模块结构模式”：
      - 若用户显式要求，或仓库为空，或现有模块已接近 `common/domain/application/infrastructure/interfaces/bootstrap`，则允许按该结构推进
      - 否则继续按真实现状建模，禁止把老项目强行改造成该结构
   h. 询问澄清问题：层级映射、特殊导入、测试偏好、全量还是部分搭建

2. **Phase 1 — AGENTS.md**（~100 行，导航地图）
   - `Read references/agents-md-template.md` 获取模板
   - 根据 Phase 0 的发现填写，不要臆造，只反映真实现状
   - 对命令区块做归一化：Gradle 项目统一写 `gradle build`、`gradle test` 等本地命令，不写 `./gradlew`
   - 详细内容指向 docs/，不要直接内联

3. **Phase 2 — docs/ system of record**
   Required:
   - 创建：`ARCHITECTURE.md`（根目录顶层领域地图，约 30 行，指向 LAYERS.md）
   - 创建：`docs/architecture/LAYERS.md`（权威分层层级 + 修复指引）
   - 创建：`docs/golden-principles/` —— `Read references/golden-principles-guide.md` 获取编写方式
   - 创建：`docs/SECURITY.md` —— `Read references/security-template.md` 获取模板与排除规则
   Recommended:
   - 创建：`docs/guides/`（按项目需要提供 setup、testing、deployment 指南）
   - 创建：`docs/exec-plans/` —— `Read references/exec-plan-template.md` 获取标准（含 `active/` + `completed/` 子目录）
   - 创建：`docs/design-docs/`，包含 `index.md`（ADR 索引）和 `core-beliefs.md`（不可违背的决策）
   - 创建：`docs/references/`（为 LLM 重排的外部文档，例如 `{library}-llms.txt`）
   - 创建：`docs/DESIGN.md`、`docs/PLANS.md`、`docs/QUALITY_SCORE.md`
   Conditional（按项目类型）:
   - `docs/RELIABILITY.md` —— 适用于服务类项目（SLA、错误预算、韧性模式）
   - `docs/STACK.md` —— 技术栈约定（替代 OpenAI 原始方案中的 FRONTEND.md）
   - `docs/product-specs/` —— 适用于产品驱动型项目
   - `docs/generated/` —— 自动生成文档（db-schema.md、api-spec.md）

4. **Phase 3 — Architecture boundary test**
   - `Read references/boundary-test-template.md` 获取测试骨架、KNOWN_VIOLATIONS 格式与棘轮逻辑
   - `Read references/stack-routing.md` 获取不同技术栈的 import parser 与测试文件路径
   - 扫描所有源码文件，解析 imports，并依据层级规则校验
   - 错误格式：`VIOLATION: {file}:{line} imports {target} — {layer} cannot import {target_layer}. See docs/architecture/LAYERS.md`
   - 棘轮机制：`KNOWN_VIOLATIONS` 对 Rust workspace 默认存放在 `architecture-tests/known-violations.json`；其他栈可用 `tests/architecture/known-violations.json`；Java/Kotlin 使用 `src/test/resources/architecture/known-violations.json`，且只能缩减
   - 对已有仓库：先建立 baseline，再开启棘轮
   - 对 Java/Kotlin 项目：必须先根据真实 package 结构识别层级；不要硬编码 `controller/service/repository/domain` 这类通用包名
   - 对老 JVM 项目：优先生成 `src/test/resources/architecture/known-violations.json` baseline，再逐步收紧，不要直接让全部历史违规失败

5. **Phase 4 — Linter boundary enforcement**
   - `Read references/stack-routing.md` 获取不同技术栈对应的 linter 规则名和配置位置
   - 对具备原生导入限制规则的技术栈，使用 linter 原生规则
   - 对 Java 项目，若无通用独立 linter，则继续使用 ArchUnit + 文档化层级规则作为边界强制手段
   - 每条错误信息都必须包含修复指引 —— 错误输出本身就是 agent 上下文

6. **Phase 5 — GC / Consistency Check**
   - `Read references/gc-patterns.md` 获取轻量一致性检查模式
   - 只面向目标仓库执行，不检查当前 skill 源仓库本身
   - 检查文档漂移：`AGENTS.md`、`ARCHITECTURE.md`、`docs/architecture/LAYERS.md` 是否仍与当前结构一致
   - 检查边界漂移：若已存在 boundary test / KNOWN_VIOLATIONS，确认没有无提示地扩大违规范围
   - 检查规则漂移：linter 规则、分层说明、目录结构是否仍然互相匹配
   - 产出一个轻量命令入口（例如 `make check-consistency`、`npm run check:consistency`、或对应栈的本地脚本），但不生成 CI workflow
   - 若仓库还太早期，可先生成检查清单或占位脚本，等结构稳定后再补强

</Steps>

<Tool_Usage>
按意图而不是具体工具调用来委派。`Read references/tool-routing.md` 获取不同平台的映射方式。

- **Explore**（轻量模型）—— 在 Phase 0 做目录映射、文件发现
- **Architect**（重型模型）—— 在 Phase 0 做架构分析和层级识别
- **Write**（轻量模型）—— 在 Phase 1-2 生成 AGENTS.md 与 docs
- **Execute**（标准模型）—— 在 Phase 3-4 实现边界测试和 linter 配置
- **Verify**（标准模型）—— 最终检查清单校验
- `references/*.md` 按 phase 需要再读，不要一开始全部加载
</Tool_Usage>

<Examples>
<Good>
User: "先别动业务代码，先把这个项目的文档和结构规范补起来"
Agent: 先执行 Phase 0 discovery -> 识别当前技术栈、目录结构、已有文档和测试 -> 判断缺失 AGENTS.md、docs/、边界约束 -> 再按默认阶段逐步补齐。
Why good: 这是客户最常见的表达方式之一，范围明确，符合先发现后落地的流程。
</Good>

<Good>
User: "把这个老项目整理成 AI 能接手维护的状态"
Agent: 先执行 Phase 0 -> 识别当前技术栈、目录结构和现有规范 -> 判断哪些文档和边界缺失 -> 对已有违规建立 baseline -> 再按 phase 逐步补齐。
Why good: 这是中文客户对“agent-ready”的典型表达，强调的是可维护性，而不是技术术语本身。
</Good>

<Good>
User: "给这个仓库加分层约束，但不要一下子改很多旧代码"
Agent: 执行 Phase 0 -> 发现大量现有 import 违规 -> 建立 KNOWN_VIOLATIONS baseline -> 先创建边界测试和 lint 规则 -> 采用渐进收敛，而不是一次性大改。
Why good: 这个说法非常贴近客户真实顾虑，skill 也正是用 baseline + ratchet 解决这个问题。
</Good>

<Good>
User: "只做 0-2 阶段，先把文档和导航补起来"
Agent: 先执行 Phase 0 -> 检测当前技术栈、目录结构与已有文档 -> 然后只创建 AGENTS.md、ARCHITECTURE.md 和 docs/ 基础结构 -> 不进入边界测试和 lint 阶段。
Why good: 很符合客户先求可读、后求约束的推进方式，也符合 phase 参数设计。
</Good>

<Good>
User: "先帮我看看这个项目适不适合补一套 AGENTS.md 和 docs"
Agent: 先执行 Phase 0 -> 检测项目大小、目录结构、现有文档、测试与 lint 现状 -> 再告诉用户适合先补哪些部分，并按发现结果推进。
Why good: 这是咨询式表达，适合客户在还没完全确定方案前先让 AI 做 Discovery。
</Good>

<Good>
User: "/harness-init 5"
Agent: 先执行 Phase 0 -> 读取当前仓库已有的 AGENTS.md、docs、边界测试和 linter 配置 -> 判断哪些文档、规则或目录说明已经漂移 -> 再只补轻量一致性检查入口或检查清单，不回头重做 1-4。
Why good: 适合仓库骨架已经成形、只想补后续维护护栏的场景，也明确体现了 Phase 5 是面向目标仓库的后置检查。
</Good>

<Bad>
User: "harness-init"
Agent: 直接生成一个 React/TypeScript 模板的 AGENTS.md，完全不看仓库。
Why bad: 跳过了 Phase 0 Discovery，在没有检测技术栈的情况下直接假设。
</Bad>

<Bad>
User: "把这个项目整理一下"
Agent: 没看代码结构就一次性生成全套 docs 和规则文件。
Why bad: “整理一下” 是模糊请求，必须先做 Discovery 并与用户确认范围。
</Bad>

<Bad>
User: "make this agent-ready" (repo has 500 lint violations)
Agent: 直接加入严格 lint 规则，让当前仓库立刻因为这 500 个违规全部失败。
Why bad: 没有先建立 baseline，直接打爆现有工作流。应先 warn，再用 ratchet 收敛。
</Bad>
</Examples>

<Escalation_And_Stop_Conditions>
- 如果技术栈检测有歧义（多个包管理器、框架不明确），**停止并询问**
- 如果目录结构不清晰，但仓库基本为空，进入空仓库初始化模式：先确认技术栈/项目类型/分层方式，再生成基础骨架并重新做 Discovery
- 如果目录结构不清晰，且不是空仓库，**停止并询问**
- 如果现有 AGENTS.md 或 docs/ 与 harness 结构冲突，**停止并询问**
- 如果 linter/test runner 无法安装（权限问题、版本不兼容），**停止并报告**
- 若 `gh` CLI、LSP 或会话状态不可用，进行**优雅降级** —— 跳过这些动态信号并注明缺失项
- **绝不能**强行套用一个不符合真实代码结构的层级模型
</Escalation_And_Stop_Conditions>

<Final_Checklist>
- [ ] 已完成 Phase 0 discovery（技术栈识别、层级识别、用户确认）
- [ ] 根目录已存在 AGENTS.md（约 100 行，作为索引而不是百科）
- [ ] 已存在 docs/architecture/LAYERS.md，包含层级图和修复指引
- [ ] 至少存在 2-3 个 golden principles 文档，含 DO/DON'T 示例
- [ ] 架构边界测试已创建并通过（已有仓库可带 KNOWN_VIOLATIONS）
- [ ] Linter 规则能强制导入边界，且错误输出带修复提示
- [ ] 已提供目标仓库的一致性检查入口或检查清单，不绑定 CI
- [ ] 测试与 linter 配置已验证可运行
</Final_Checklist>

<Advanced>
## Target File Structure

```
project-root/
├── AGENTS.md                          # ~100 lines, orientation map          [Required]
├── ARCHITECTURE.md                    # Top-level domain map                 [Required]
├── docs/
│   ├── architecture/
│   │   └── LAYERS.md                  # Layer hierarchy + enforcement        [Required]
│   ├── golden-principles/             # DO/DON'T patterns, 30-60 lines each [Required]
│   ├── SECURITY.md                    # Auth, secrets, threat model          [Required]
│   ├── guides/                        # Setup, testing, deployment           [Recommended]
│   ├── exec-plans/                    # ExecPlan lifecycle                   [Recommended]
│   │   ├── active/
│   │   ├── completed/
│   │   └── tech-debt-tracker.md
│   ├── design-docs/                   # ADRs                                [Recommended]
│   │   ├── index.md
│   │   ├── core-beliefs.md
│   │   └── {NNNN-title}.md
│   ├── references/                    # External docs for LLMs              [Recommended]
│   │   └── {library}-llms.txt
│   ├── DESIGN.md                      # Design philosophy                   [Recommended]
│   ├── PLANS.md                       # Exec-plans overview                 [Recommended]
│   ├── QUALITY_SCORE.md               # Per-domain quality grades           [Recommended]
│   ├── RELIABILITY.md                 # SLA, error budgets (services)       [Conditional]
│   ├── STACK.md                       # Stack conventions                   [Conditional]
│   ├── product-specs/                 # Product specs                       [Conditional]
│   └── generated/                     # Auto-generated docs                 [Conditional]
│       └── {db-schema,api-spec}.md
├── architecture-tests/                # Rust workspace boundary tests       [Rust]
│   ├── known-violations.json
│   └── tests/boundary_test.rs
```

## Reference Files

详细模板与指南位于 `references/`，按 phase 需要再读：
- `references/layer-templates.md` —— 5 种分层模型（4 个技术栈 + OpenAI 原始模型）
- `references/agents-md-template.md` —— AGENTS.md 模板
- `references/context-strategy.md` —— 静态/动态上下文表
- `references/exec-plan-template.md` —— ExecPlan（docs/exec-plans/）标准
- `references/golden-principles-guide.md` —— golden principles 编写指南
- `references/security-template.md` —— 含排除规则的 SECURITY.md 模板
- `references/boundary-test-template.md` —— 测试骨架、KNOWN_VIOLATIONS 格式、棘轮逻辑
- `references/gc-patterns.md` —— 轻量一致性检查模式与产出建议
- `references/tool-routing.md` —— 平台级工具委派映射
- `references/stack-routing.md` —— Phase 3-4 的 技术栈 → 工具 决策表
</Advanced>
