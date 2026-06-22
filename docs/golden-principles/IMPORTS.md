# Crate 依赖与导入

## Rule

Crate 间依赖必须遵循 `docs/architecture/LAYERS.md` 的允许矩阵；同 crate 内优先 `crate::` 路径，跨 crate 只通过 Cargo.toml 声明的 workspace 依赖。

## DO

```rust
// gateway/src/adapter.rs — adapter 层只依赖 core
use hi_core::Result;

// app/src/main.rs — entry 层组装各 crate
use hi_core::Config;
// hi_tui::run_placeholder()?
```

```toml
# gateway/Cargo.toml — 仅声明允许的 foundation 依赖
[dependencies]
hi-core = { workspace = true }
```

## DON'T

```rust
// core/src/lib.rs — foundation 不得向上依赖
use hi_tui::run_placeholder; // ❌

// tui/src/lib.rs — adapter 之间不得互依赖
use hi_gateway::ChannelAdapter; // ❌
```

```toml
# core/Cargo.toml — foundation 不得依赖其他 hi crate
[dependencies]
hi-ai = { workspace = true }  # ❌ Provider 由 app 层组装
```

## Exceptions

- 测试 crate `architecture-tests` 可读取全部 crate 源码用于边界扫描，但自身 `[dependencies]` 不引入业务 crate
