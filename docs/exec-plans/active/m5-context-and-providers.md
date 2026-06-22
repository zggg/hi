# M5：上下文压缩 + 多 Provider

This ExecPlan is a living document.

## Purpose

- 对话历史超过 `context.compress_at_k`（K token）时自动摘要压缩
- 支持 `openai-compat` / `anthropic` / `ollama` 三种 Provider

## Progress

- [x] `[context]` 配置段
- [x] `core/src/context.rs` — token 估算 + `maybe_compress`
- [x] `AgentLoop` 集成 + `ContextCompressed` 事件
- [x] `SessionStore::replace_messages` 压缩后落库
- [x] `AnthropicProvider` + `OllamaProvider`
- [x] `runtime.rs` provider 工厂

## Validation

```sh
# 压缩：调低 max_tokens 或拉长对话后观察 [context compressed]
hi config

hi chat 你好

# Anthropic：hi setup 选择 anthropic，api_key 写入 hi.toml

# Ollama
# ai.provider = "ollama", ollama run llama3.2
```
