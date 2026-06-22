# 命名约定

## Rule

Crate 目录名与 package 名对齐；Rust 标识符用 snake_case；对外二进制名与产品名一致为 `hi`。

## DO

| 概念 | 约定 | 示例 |
|------|------|------|
| 目录 | 职责名（小写） | `core/`, `ai/`, `app/` |
| Package | `hi-*` 前缀（app 除外） | `hi-core`, `hi-ai`, `hi` |
| 模块 | snake_case | `tools/`, `store/` |
| 类型 | PascalCase | `AgentEvent`, `SessionId` |
| 常量 | SCREAMING_SNAKE | `TOOL_READ` |

```rust
// core/src/tools/builtin.rs
pub const TOOL_READ: &str = "read";
pub struct ToolRegistry { ... }
```

## DON'T

```
hi-core/          # ❌ 目录不要用 package 名的连字符形式
HiCore/           # ❌
src/HiCore.rs     # ❌ Rust 模块文件用 snake_case
```

```rust
// 不要在 app crate 中重新 export 整个 core 公共 API 除非必要
pub use hi_core::*;  // ❌
```

## Exceptions

- `app/` 目录对应 package `hi`（二进制名 `hi`）；见 README 与 AGENTS.md
