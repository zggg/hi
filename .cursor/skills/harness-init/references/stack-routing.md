# 技术栈路由 —— Phases 3-4 的决策表

用这些表为已识别出的技术栈选择正确工具。Phase 0 负责识别技术栈，这些表负责决定执行方式。

## Phase 3: Boundary Test — Import Parser & Pattern

| Stack | Import Pattern | Parser Approach | Test File |
|-------|---------------|-----------------|-----------|
| JS/TS | `import ... from '...'` | Regex or AST (ts-morph, babel) | `tests/architecture/boundary.test.ts` |
| Python | `import ...` / `from ... import` | AST (stdlib `ast` module) | `tests/architecture/test_boundary.py` |
| Go | `import "..."` | `go/parser` stdlib or regex | `tests/architecture/boundary_test.go` |
| Rust | `use ...` / `mod ...` | Regex or `syn` crate (heavier) | `architecture-tests/tests/boundary_test.rs` + `architecture-tests/known-violations.json` |
| Java/Kotlin | `import ...` | Regex or ArchUnit（基于真实 package 结构） | `src/test/java/.../architecture/BoundaryTest.java` |

**统一错误格式：**
`VIOLATION: {file}:{line} imports {target} — {layer} cannot import {target_layer}. See docs/architecture/LAYERS.md`

## Phase 4: Linter Import Restriction Rules

| Stack | Linter | Rule | Config Location |
|-------|--------|------|-----------------|
| JS/TS (ESLint) | eslint | `no-restricted-imports` / `import/no-restricted-paths` | `.eslintrc` or `eslint.config.js` |
| Python (Ruff) | ruff | `banned-api` (flake8-tidy-imports) | `pyproject.toml [tool.ruff]` |
| Python (Flake8) | flake8 | `flake8-import-restrictions` | `.flake8` or `setup.cfg` |
| Go | golangci-lint | `depguard` | `.golangci.yml` |
| Rust | clippy | `pub(crate)` visibility + workspace deps | `Cargo.toml` + module structure |
| Java | 无通用独立 linter；继续使用 ArchUnit + 文档化层级规则 | `ArchRuleDefinition.noClasses()` / 自定义依赖扫描 | `src/test/java/.../architecture/BoundaryTest.java` |

**关键规则：** 每条 linter 错误都必须包含 remediation 文本。错误输出本身就是 agent 上下文。

## JVM 特别说明

- 对 Java/Kotlin 项目，优先根据真实 package 命名识别层，而不是套 `controller/service/repository/domain` 通用词表
- 老项目应先建 `src/test/resources/architecture/known-violations.json` baseline，再逐步收紧；不要直接让所有历史违规一次性失败
- 生成 ArchUnit 测试前，先确认项目是否真的采用了分层 package 结构
- Spring Boot + Gradle 项目优先检查：`build.gradle` / `build.gradle.kts`、`src/main/java`、`src/test/java`
- 单模块项目优先把 BoundaryTest 放在 `src/test/java/{basePackage}/architecture/`
- 多模块项目先确认边界是“模块内分层”还是“模块间依赖”，不要混成一套规则
- 需要补测试依赖时，优先使用本地 `gradle test` 可执行的依赖组合：`archunit-junit5` + `junit-jupiter`
- Phase 4 对 Java 项目通常不生成独立 linter，而是把“边界强制”继续落在 ArchUnit 与文档化层级规则上；除非仓库已有额外静态检查体系
- 如果目标是老项目且结构明显不符合标准多模块模式，不要把边界测试建立在 `common/domain/application/infrastructure/interfaces/bootstrap` 假设上
