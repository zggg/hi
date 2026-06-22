# 错误处理

## Rule

库 crate（`hi-core`、`hi-ai`、`hi-gateway`、`hi-tui`）使用 crate 内定义的 `Error` / `Result`；二进制入口（`hi`）使用 `anyhow` 做顶层错误聚合与打印。

## DO

```rust
// core/src/error.rs
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
}
pub type Result<T> = std::result::Result<T, Error>;

// gateway/src/lib.rs
pub fn run_placeholder(bind: &str) -> hi_core::Result<()> { ... }

// app/src/main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> { ... }
```

边界测试与 lint 报错应包含修复指引，例如：

```
VIOLATION: ... See docs/architecture/LAYERS.md
```

## DON'T

```rust
// 在 hi-core 中直接 panic 处理可恢复错误
.unwrap()  // ❌ 除非原型阶段且明确标注

// 在 adapter 层定义与 core 重复的错误类型
pub enum GatewayError { ... }  // ❌ 优先复用 hi_core::Error
```

## Exceptions

- `bash` 工具执行失败时，错误信息应包含命令与 stderr 摘要，供 TUI/Gateway 展示给用户
