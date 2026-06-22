# Boundary Test 模板

用于 Phase 3。提供 KNOWN_VIOLATIONS 格式、棘轮逻辑和测试骨架。

## KNOWN_VIOLATIONS 格式

File: `tests/architecture/known-violations.json`

```json
[
  {
    "file": "src/components/UserCard.tsx",
    "line": 5,
    "imports": "src/services/userService",
    "from_layer": "components",
    "to_layer": "services",
    "reason": "Legacy coupling — tracked for removal in Q2"
  }
]
```

**规则：**
- 每条违规对应一个条目。每个条目由 `file` + `imports` 唯一标识。
- 一旦 baseline 建立，条目只能**删除**（表示已修复），不能新增。
- 如果出现列表中不存在的新违规，棘轮测试必须失败。
- 违规修复后，要删除对应条目。总数量只能减少。

## 层级定义

在测试文件或配置文件中定义每层允许导入哪些层：

```json
{
  "types": [],
  "lib": ["types"],
  "services": ["lib", "types"],
  "components": ["lib", "types"],
  "pages": ["components", "services", "lib", "types"]
}
```

每个 key 代表一层，value 是它允许导入的层。超出该集合的任何导入都算违规。

## 测试骨架：TypeScript（Jest/Vitest）

```typescript
import { readdirSync, readFileSync } from 'fs';
import { join, relative } from 'path';
import knownViolations from './known-violations.json';

const LAYER_RULES: Record<string, string[]> = {
  types: [],
  lib: ['types'],
  services: ['lib', 'types'],
  components: ['lib', 'types'],
  pages: ['components', 'services', 'lib', 'types'],
};

// Matches `from '...'` on any line — handles single-line and multi-line imports,
// plus re-exports (`export { x } from '...'`).
// NOTE: This is a simplified scanner. For full accuracy (dynamic imports, require()),
// use ts-morph or @babel/parser AST parsing (see references/stack-routing.md).
const FROM_RE = /\bfrom\s+['"]([^'"]+)['"]/;

function getLayer(filePath: string): string | null {
  // Adapt 'src' to your project's source root
  const match = filePath.match(/^src\/([^/]+)\//);
  if (match && match[1] in LAYER_RULES) return match[1];
  return null;
}

function resolveTargetLayer(importPath: string): string | null {
  // Strip common path aliases (@/, ~/, #/)
  const normalized = importPath.replace(/^[@~#]\//, '');
  const segments = normalized.split('/');
  for (const layer of Object.keys(LAYER_RULES)) {
    if (segments.includes(layer)) return layer;
  }
  return null;
}

function scanFile(filePath: string): Array<{ file: string; line: number; imports: string; from_layer: string; to_layer: string }> {
  const violations: Array<{ file: string; line: number; imports: string; from_layer: string; to_layer: string }> = [];
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const fromLayer = getLayer(relative(process.cwd(), filePath));
  if (!fromLayer) return violations;

  let inTypeImport = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Track type-only imports (erased at compile time, not runtime dependencies)
    if (/^\s*import\s+type\s/.test(line)) inTypeImport = true;

    const match = line.match(FROM_RE);
    if (match && !inTypeImport) {
      const targetLayer = resolveTargetLayer(match[1]);
      if (targetLayer && !LAYER_RULES[fromLayer].includes(targetLayer) && targetLayer !== fromLayer) {
        violations.push({
          file: relative(process.cwd(), filePath),
          line: i + 1,
          imports: match[1],
          from_layer: fromLayer,
          to_layer: targetLayer,
        });
      }
    }

    // Reset type-import tracking when the from-clause ends the statement
    if (inTypeImport && FROM_RE.test(line)) inTypeImport = false;
  }
  return violations;
}

// Collect all source files recursively
function collectFiles(dir: string, ext: string[]): string[] {
  const results: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      results.push(...collectFiles(fullPath, ext));
    } else if (ext.some(e => entry.name.endsWith(e))) {
      results.push(fullPath);
    }
  }
  return results;
}

describe('Architecture Boundary Test', () => {
  // Adapt 'src' to your project's source root
  const files = collectFiles('src', ['.ts', '.tsx']);
  const allViolations = files.flatMap(scanFile);

  test('no new architecture violations', () => {
    const knownSet = new Set(knownViolations.map(v => `${v.file}:${v.imports}`));
    const newViolations = allViolations.filter(v => !knownSet.has(`${v.file}:${v.imports}`));

    if (newViolations.length > 0) {
      const msg = newViolations
        .map(v => `VIOLATION: ${v.file}:${v.line} imports ${v.imports} — ${v.from_layer} cannot import ${v.to_layer}. See docs/architecture/LAYERS.md`)
        .join('\n');
      throw new Error(`New architecture violations found:\n${msg}`);
    }
  });

  test('violation count only shrinks (ratchet)', () => {
    expect(allViolations.length).toBeLessThanOrEqual(knownViolations.length);
  });
});
```

## 测试骨架：Python（pytest）

```python
import ast
import json
from pathlib import Path

LAYER_RULES = {
    "models": [],
    "config": ["models"],
    "db": ["config", "models"],
    "services": ["db", "config", "models"],
    "middleware": ["services", "config", "models"],
    "routes": ["services", "middleware", "models"],
}

KNOWN_VIOLATIONS_PATH = Path("tests/architecture/known-violations.json")


def get_layer(file_path: Path) -> str | None:
    # Adapt 'src' to your project's source root (e.g., package name, 'app/')
    parts = file_path.parts
    if len(parts) >= 2 and parts[0] == "src" and parts[1] in LAYER_RULES:
        return parts[1]
    return None


def scan_imports(file_path: Path) -> list[dict]:
    violations = []
    source = file_path.read_text()
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return violations  # Skip unparseable files
    from_layer = get_layer(file_path)
    if not from_layer:
        return violations

    def _check_target(target: str, node: ast.AST) -> None:
        for layer in LAYER_RULES:
            if layer in target.split("."):
                if layer != from_layer and layer not in LAYER_RULES[from_layer]:
                    violations.append({
                        "file": str(file_path),
                        "line": node.lineno,
                        "imports": target,
                        "from_layer": from_layer,
                        "to_layer": layer,
                    })
                break  # Only match the first (shallowest) layer

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                _check_target(alias.name, node)
        elif isinstance(node, ast.ImportFrom) and node.module:
            _check_target(node.module, node)

    return violations


def test_no_new_violations():
    known = json.loads(KNOWN_VIOLATIONS_PATH.read_text()) if KNOWN_VIOLATIONS_PATH.exists() else []
    known_set = {(v["file"], v["imports"]) for v in known}

    # Adapt 'src' to your project's source root (e.g., package name, 'app/')
    all_violations = []
    for py_file in Path("src").rglob("*.py"):
        all_violations.extend(scan_imports(py_file))

    new_violations = [v for v in all_violations if (v["file"], v["imports"]) not in known_set]
    assert not new_violations, "\n".join(
        f"VIOLATION: {v['file']}:{v['line']} imports {v['imports']} — "
        f"{v['from_layer']} cannot import {v['to_layer']}. See docs/architecture/LAYERS.md"
        for v in new_violations
    )


def test_ratchet_only_shrinks():
    known = json.loads(KNOWN_VIOLATIONS_PATH.read_text()) if KNOWN_VIOLATIONS_PATH.exists() else []

    # Adapt 'src' to your project's source root (e.g., package name, 'app/')
    all_violations = []
    for py_file in Path("src").rglob("*.py"):
        all_violations.extend(scan_imports(py_file))

    assert len(all_violations) <= len(known), (
        f"Violation count increased: {len(all_violations)} > baseline {len(known)}. "
        "Fix violations to reduce the count — never add new ones."
    )
```

## 为已有仓库建立 Baseline

1. 在没有 `known-violations.json` 的情况下先运行 boundary test —— 它会报告当前所有违规
2. 将输出整理写入 `known-violations.json`，作为初始 baseline
3. 提交这个 baseline —— 这就是棘轮的起点
4. 从此以后，违规数量只能下降

## 测试骨架：Java / Kotlin（ArchUnit）

适用于已有明确 package 结构的 JVM 项目。**不要**直接硬编码 `controller/service/repository/domain/po` 这一类常见包名；必须先根据真实目录和 import 关系确定层级。

### 生成前要求

- 先从真实源码中识别基础包名（例如 `com.example.app`）
- 先确认项目里的真实层名（例如 `web`、`application`、`domain`、`infrastructure`，或团队自定义命名）
- 老项目先建立 baseline，不要一上来把历史违规全部打爆
- 如果是 Spring Boot + Gradle 项目，先检查：
  - `build.gradle` / `build.gradle.kts`
  - `src/main/java/{basePackage}`
  - `src/test/java/{basePackage}`
  - 是否已有 `architecture/`、`archunit/`、`layer` 相关测试

### 推荐文件

- 测试类：`src/test/java/.../architecture/BoundaryTest.java`
- baseline：`src/test/resources/architecture/known-violations.json`

### 依赖

至少需要：

- `com.tngtech.archunit:archunit-junit5`
- `org.junit.jupiter:junit-jupiter`
- 一个 JSON 解析器（例如 `com.fasterxml.jackson.core:jackson-databind`）

Gradle 示例（Groovy DSL）：

```gradle
dependencies {
    testImplementation 'com.tngtech.archunit:archunit-junit5:1.3.0'
    testImplementation 'org.junit.jupiter:junit-jupiter:5.10.2'
    testImplementation 'com.fasterxml.jackson.core:jackson-databind:2.17.2'
}
```

Gradle 示例（Kotlin DSL）：

```kotlin
dependencies {
    testImplementation("com.tngtech.archunit:archunit-junit5:1.3.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.2")
    testImplementation("com.fasterxml.jackson.core:jackson-databind:2.17.2")
}
```

### Baseline 格式

JVM 项目统一使用结构化 JSON，而不是文本清单：

```json
[
  {
    "rule_id": "infrastructure_should_not_depend_on_web",
    "source_class": "com.example.app.infrastructure.UserRepository",
    "target_class": "com.example.app.web.UserController",
    "source_layer": "infrastructure",
    "target_layer": "web",
    "reason": "Legacy coupling — to be removed after module split"
  }
]
```

字段要求：

- `rule_id`：对应哪条 ArchUnit 规则
- `source_class`：违规来源类全名
- `target_class`：违规目标类全名
- `source_layer`：来源层
- `target_layer`：目标层
- `reason`：为什么暂时允许存在

不要只记录文本错误消息；baseline 必须是可解析、可比对的结构化文件。

### 示例骨架

```java
package com.example.app.architecture;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.tngtech.archunit.core.domain.Dependency;
import com.tngtech.archunit.core.domain.JavaClass;
import com.tngtech.archunit.core.domain.JavaClasses;
import com.tngtech.archunit.core.importer.ClassFileImporter;
import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.io.InputStream;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.stream.Collectors;

class BoundaryTest {

    private static final String BASE_PACKAGE = "com.example.app";
    private static final JavaClasses CLASSES =
            new ClassFileImporter().importPackages(BASE_PACKAGE);
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();

    @Test
    void infrastructure_should_not_depend_on_web() {
        List<Violation> violations = collectViolations(
                "infrastructure_should_not_depend_on_web",
                "infrastructure",
                "web",
                BASE_PACKAGE + ".infrastructure..",
                BASE_PACKAGE + ".web.."
        );
        List<Violation> baseline = loadBaseline();

        assertNoNewViolations(violations, baseline);
    }

    private static List<Violation> collectViolations(
            String ruleId,
            String sourceLayer,
            String targetLayer,
            String sourcePackage,
            String targetPackage
    ) {
        List<Violation> violations = new ArrayList<>();
        for (JavaClass javaClass : CLASSES) {
            if (!matchesPackage(javaClass.getPackageName(), sourcePackage)) {
                continue;
            }
            for (Dependency dependency : javaClass.getDirectDependenciesFromSelf()) {
                JavaClass targetClass = dependency.getTargetClass();
                if (matchesPackage(targetClass.getPackageName(), targetPackage)) {
                    violations.add(new Violation(
                            ruleId,
                            javaClass.getName(),
                            targetClass.getName(),
                            sourceLayer,
                            targetLayer,
                            "legacy"
                    ));
                }
            }
        }
        return violations;
    }

    private static boolean matchesPackage(String packageName, String packagePattern) {
        String normalized = packagePattern.replace("..", "");
        if (normalized.endsWith(".")) {
            normalized = normalized.substring(0, normalized.length() - 1);
        }
        return packageName.startsWith(normalized);
    }

    private static List<Violation> loadBaseline() {
        try (InputStream input = BoundaryTest.class.getResourceAsStream("/architecture/known-violations.json")) {
            if (input == null) {
                return List.of();
            }
            return OBJECT_MAPPER.readValue(input, new TypeReference<List<Violation>>() {});
        } catch (IOException e) {
            throw new RuntimeException("Failed to read known-violations.json", e);
        }
    }

    private static void assertNoNewViolations(List<Violation> violations, List<Violation> baseline) {
        Set<String> known = baseline.stream()
                .map(Violation::key)
                .collect(Collectors.toSet());

        List<Violation> newViolations = violations.stream()
                .filter(v -> !known.contains(v.key()))
                .toList();

        if (!newViolations.isEmpty()) {
            String message = newViolations.stream()
                    .map(v -> "VIOLATION: " + v.sourceClass() + " depends on " + v.targetClass()
                            + " — " + v.sourceLayer() + " cannot import " + v.targetLayer()
                            + ". See docs/architecture/LAYERS.md")
                    .collect(Collectors.joining("\n"));
            throw new AssertionError("New architecture violations found:\n" + message);
        }
    }

record Violation(
            String ruleId,
            String sourceClass,
            String targetClass,
            String sourceLayer,
            String targetLayer,
            String reason
    ) {
        String key() {
            return ruleId + "::" + sourceClass + "->" + targetClass;
        }
    }
}
```

### 首次生成 baseline

第一次在老项目落地时，不要手工从报错文本整理。推荐做法是：

1. 先按真实层级把 ArchUnit 规则写好
2. 临时运行一次“收集模式”，把当前全部违规导出为 JSON
3. 审核 `known-violations.json`
4. 提交 baseline
5. 之后切回正常模式，只允许新增违规失败

多规则聚合导出示例：

```java
import java.nio.file.Files;
import java.nio.file.Path;

@Test
void export_initial_baseline() throws Exception {
    List<Violation> violations = new ArrayList<>();
    violations.addAll(collectViolations(
            "infrastructure_should_not_depend_on_web",
            "infrastructure",
            "web",
            BASE_PACKAGE + ".infrastructure..",
            BASE_PACKAGE + ".web.."
    ));
    violations.addAll(collectViolations(
            "domain_should_not_depend_on_web",
            "domain",
            "web",
            BASE_PACKAGE + ".domain..",
            BASE_PACKAGE + ".web.."
    ));
    violations.addAll(collectViolations(
            "repository_should_not_depend_on_service",
            "repository",
            "service",
            BASE_PACKAGE + ".repository..",
            BASE_PACKAGE + ".service.."
    ));

    Path output = Path.of("src/test/resources/architecture/known-violations.json");
    Files.createDirectories(output.getParent());
    OBJECT_MAPPER.writerWithDefaultPrettyPrinter().writeValue(output.toFile(), violations);
}
```

建议：

- 这个导出测试只在初始化 baseline 时临时使用
- baseline 提交后，应删除或禁用导出测试，避免覆盖人工维护过的 `reason`
- 如果有多条规则，把各条规则聚合到同一个 `List<Violation>` 后再一次性输出

推荐流程：

```text
第一次初始化：
1. export_initial_baseline
2. 审核 known-violations.json
3. 提交 baseline

后续日常使用：
1. 运行 BoundaryTest
2. 只允许新违规失败
3. 修掉旧违规后，从 baseline 删除对应条目
```

### JVM 项目特别规则

- 包规则必须来源于真实项目结构，不要拍脑袋生成
- 若项目包名不是分层式，而是按业务域组织，先停下来，不要强行产出 ArchUnit 规则
- 如果已有大量历史违规，先生成 `known-violations.json` baseline，再逐步收紧
- 不要把 Spring `configuration`、`security`、`handler`、`holder`、`converter` 自动判定成固定层，除非项目本身已经明确这样组织
- 对 Spring Boot 项目，优先先判定这些包在你们项目里扮演什么角色，再决定是否纳入边界：
  - `configuration`
  - `security`
  - `converter` / `mapper`
  - `handler`
  - `facade`
  - `client` / `integration`
