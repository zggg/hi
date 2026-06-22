# 技术栈约定（STACK）

## 语言与构建

| 项 | 选型 | 说明 |
|----|------|------|
| 语言 | Rust 2021 | Workspace 统一 edition |
| 包管理 | Cargo | 5 crate workspace |
| 异步运行时 | tokio full | CLI、Gateway、Agent 循环 |

## 各 Crate 技术选型

| Crate | 主要依赖 |
|-------|---------|
| hi-core | serde, thiserror, tokio, tracing, rusqlite |
| hi-ai | async-trait, serde, reqwest |
| hi-tui | tokio, tracing, ratatui, crossterm |
| hi-gateway | async-trait, tokio, tokio-tungstenite, futures-util |
| hi (app) | clap, tracing-subscriber, hi-core, hi-ai, hi-tui, hi-gateway |

## Workspace 依赖管理

- 共享依赖声明在根 `Cargo.toml` 的 `[workspace.dependencies]`
- 各 crate 通过 `{ workspace = true }` 引用
- 新增 hi 内部 crate 依赖必须过 LAYERS.md 矩阵

## Lint 与测试

```sh
cargo clippy --workspace -- -D warnings
cargo test --workspace
./scripts/check-consistency.sh
```

## 配置

- 路径：`~/.hi/hi.toml`（环境变量 `HI_TOML` 可覆盖）
- 格式：TOML + serde
- 加载：`hi_core::Config::load()`；`hi setup` / `hi gateway setup` 交互写入

## 日志

- `tracing` + `tracing-subscriber`
- 环境过滤：`RUST_LOG=hi=debug cargo run -p hi -- tui`
- Gateway 后台：`~/.hi/logs/hi.log`（按日切割）
