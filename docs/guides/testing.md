# 测试指南

## 架构边界测试

```sh
cargo test -p architecture-tests
```

- 测试文件：`architecture-tests/tests/boundary_test.rs`
- Baseline：`architecture-tests/known-violations.json`
- 规则来源：`docs/architecture/LAYERS.md`

### 棘轮机制

- `known-violations.json` 中的条目只能**删除**（修复后），不能新增
- 出现不在 baseline 中的新违规 → 测试失败

### 首次建立 baseline（已有违规时）

1. 运行测试查看违规输出
2. 将条目写入 `architecture-tests/known-violations.json`
3. 提交 baseline
4. 之后只允许违规数量减少

## 单元测试

业务源码（`src/`）末尾只保留两行挂载点：

```rust
#[cfg(test)]
#[path = "../../test/unit/config/gateway.rs"]
mod tests;
```

测试实现放在 **`{crate}/test/unit/`**，目录镜像 `src/`：

| 源码 | 测试 |
|------|------|
| `core/src/agent.rs` | `core/test/unit/agent.rs` |
| `app/src/config/setup.rs` | `app/test/unit/config/setup.rs` |

- 仅在 `cargo test` 时编译（`#[cfg(test)]`），**不会**进 release 二进制
- 测试文件内仍可用 `use super::*` 访问同模块私有函数（与原先内联 `mod tests` 等价）
- 新增测试：在 `test/unit/` 建镜像文件，源码末尾只保留 `#[cfg(test)]` + `#[path = "..."]` + `mod tests;` 两行挂载点

## 集成 / 架构测试

- 架构边界：`architecture-tests/tests/`
- Rust 约定 `{crate}/tests/*.rs` 为**集成测试** crate（测 public API），与本仓库 `test/unit/` 单元测试分开

## Lint

```sh
cargo clippy --workspace -- -D warnings
```

## 一致性检查

```sh
./scripts/check-consistency.sh
```

检查文档与目录结构是否漂移。

## CI（未配置）

本仓库暂不绑定 CI；本地提交前运行上述命令即可。
