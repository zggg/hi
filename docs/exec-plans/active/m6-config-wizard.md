# M6：配置向导与首个可用版本

This ExecPlan is a living document.

## Purpose

- `hi setup` 交互式生成 `~/.hi/hi.toml`
- 文档与示例对齐 M5 能力

## Progress

- [x] `Config::save()`
- [x] `hi setup` 向导
- [x] `hi.example.toml` 含 `[context]`
- [x] README / setup 更新

## Validation

```sh
hi setup
hi config
hi chat 你好
hi tui
```
