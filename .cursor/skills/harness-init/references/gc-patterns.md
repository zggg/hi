# 轻量 GC / 一致性检查模式

这里的 GC 指 **目标仓库的一致性与漂移检查**，不是内存垃圾回收，也不是当前 skill 源仓库的自检。

## 目标

- 发现文档与实际结构之间的漂移
- 发现分层规则与导入关系之间的不一致
- 给目标仓库补一个可重复执行的轻量检查入口
- 不绑定 CI，不要求定时任务，不引入平台依赖

## 检查范围

### 1. 文档漂移

检查这些文件是否仍与当前项目现状一致：

- `AGENTS.md`
- `ARCHITECTURE.md`
- `docs/architecture/LAYERS.md`
- `docs/golden-principles/*.md`

重点看：

- 目录名是否变了
- 关键模块是否新增但文档未提及
- 关键约束是否已失效

### 2. 边界漂移

如果仓库已经有边界测试或 `KNOWN_VIOLATIONS`：

- 检查违规数量是否意外扩大
- 检查新增目录是否未纳入边界规则
- 检查 LAYERS.md 与测试目标是否仍对应

### 3. 规则漂移

检查这些内容是否还互相匹配：

- linter 配置
- 边界测试
- `docs/architecture/LAYERS.md`
- 实际目录结构

## 推荐产物

根据目标仓库技术栈，选择一个最轻的本地入口：

- `make check-consistency`
- `npm run check:consistency`
- `pnpm check:consistency`
- `python scripts/check_consistency.py`
- `bash scripts/check-consistency.sh`

如果项目还太早期，没有合适的脚本位置：

- 先创建 `docs/guides/consistency-check.md`
- 或在 `AGENTS.md` 中加入“当前一致性检查方式”说明

## 输出要求

- 报错要指出具体漂移点，而不是只返回 pass/fail
- 优先给出修复动作，例如“更新 LAYERS.md”“同步 AGENTS.md 命令区块”
- 不要求一次性修完历史问题，但要明确当前差异

## 不要做的事

- 不要生成 `.github/workflows/*`
- 不要绑定 GitHub / GitLab / 平台级流水线
- 不要把当前 skill 源仓库的 `scripts/gc/check-consistency.sh` 直接复制到目标仓库
- 不要把 GC 设计成独立的大型治理系统
