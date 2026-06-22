# AGENTS.md 模板

约 100 行。它应该是索引，而不是百科全书。那种 800 行版本之所以被淘汰，是因为 agent 找不到真正需要的信息。

~~~markdown
# {Project Name} — Agent 导航地图

> {一行描述}

## 技术栈

| Layer     | Tech       |
|-----------|------------|
| Language  | {version}  |
| Framework | {version}  |
| Database  | {type}     |

## 架构分层

依赖只能**向下流动**。禁止向上导入。

{从真实 import 模式中发现的层级图}

## 关键约定

- {约定 1 —— 简要说明，并指向 docs/golden-principles/}
- {约定 2}
- {约定 3}

## 命令

```sh
{build_command}
{test_command}
{lint_command}
{dev_command}
```

命令生成规则：
- 优先使用项目团队约定的主命令形式
- 如果是 Gradle 项目，默认写本地 `gradle ...` 命令，不写 `./gradlew ...`
- 只有用户明确要求保留 wrapper 时，才使用 `./gradlew`

## 文档地图

```
ARCHITECTURE.md                       顶层领域地图（根目录）
docs/
├── architecture/                     分层规则、依赖图
├── golden-principles/                典范模式（DO/DON'T 示例）
├── SECURITY.md                       认证、密钥、威胁模型
├── guides/                           setup、testing、deployment 指南
├── exec-plans/                       功能实施计划
├── design-docs/                      架构决策记录
└── references/                       外部库文档（LLM 友好格式）
```

## 从哪里开始看

| 任务 | 先看这里 |
|-------------------|-------------------------------|
| 架构概览 | ARCHITECTURE.md（根目录） |
| 分层规则 | docs/architecture/LAYERS.md |
| {常见任务 1} | {directory/file} |
| {常见任务 2} | {directory/file} |
| {常见任务 3} | {directory/file} |

## 约束（机器可读）

- MUST: {硬规则 + 对应强制方式}
- MUST NOT: {禁止项 + 指向 LAYERS.md}
- PREFER: {软偏好}
- VERIFY: {提交前验证命令}
~~~
