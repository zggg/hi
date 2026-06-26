# hi

[![CI](https://github.com/zggg/hi/actions/workflows/ci.yml/badge.svg)](https://github.com/zggg/hi/actions/workflows/ci.yml)

中文: [README.zh.md](README.zh.md)

**Rust AI agent** — ultra-lightweight personal assistant: **single binary** · local TUI · optional IM gateway · one shared Agent core

> **Personal learning project** — for practicing Rust, agent runtimes, TUI, and messaging-channel integration. Features evolve continuously; **not production-ready**. Feedback welcome; do not rely on it for critical workloads.

## Why hi

- **Tiny & instant** — release binary ~10 MB, no extra runtime; cold start straight into `hi chat` / `hi tui`
- **Lightweight** — four core tools + optional memory, no MCP/Skills stack; one config file
- **One core, many surfaces** — TUI, terminal chat, HTTP API, WeCom / Feishu / WeChat iLink share the same Agent; sessions isolated per channel
- **Local-first** — SQLite persistence, unified `~/.hi/hi.toml`, secrets stay on your machine
- **Reviewable & controllable** — bash and out-of-scope file access require approval by default; dangerous commands are hardline and cannot be granted
- **Multi-provider** — OpenAI-compatible · Anthropic · Ollama · Codex; gateway uses long-lived connections, no public callback URL required

## Quick start

```bash
cargo install --path app
hi setup
hi chat hello          # or hi / hi tui
```

### npm

```bash
npm install -g @i99/hi
hi setup
```

Docs: [Install](docs/guides/install.md) · [Architecture](ARCHITECTURE.md) · [Security](docs/SECURITY.md)

## Tools

The agent ships 4 core tools, plus 2 more when memory is enabled (gated by `[memory]` config):

| Tool | Purpose | Approval |
|------|---------|----------|
| `read` | Read a file; relative paths resolve from the working directory | Out-of-workspace paths need one-time approval (per directory tree) |
| `write` | Write a file (full overwrite) | Out-of-workspace paths need one-time approval |
| `edit` | Replace the first occurrence of `old_string` in a file | Out-of-workspace paths need one-time approval |
| `bash` | Run a shell command in the working directory | Dangerous commands and out-of-workspace writes need one-time approval; hardline commands cannot be granted |
| `memory_search` *(optional)* | Search long-term knot memory by keyword (tasks / decisions / procedures / facts) | — |
| `memory_write` *(optional)* | Save durable memory worth keeping across sessions; de-duplicated automatically | — |

> Approvals are stored in `tools.approvals`; set `mode = "off"` to disable them. `memory_search` / `memory_write` are gated by `memory_search_enabled` / `memory_write_tool` respectively.

## Commands

| Command | Description |
|---------|-------------|
| `hi` / `hi tui` | Start the local TUI (`-s <session>` to pick a session, `-v` for verbose) |
| `hi chat [message]` | Terminal chat: with args = single turn, without = stdin REPL |
| `hi setup` | Setup wizard: LLM + workspace (re-run keeps current values on Enter) |
| `hi model` | Configure the model only: add / switch model, keep workspace & channels |
| `hi config` | Print effective configuration (secrets redacted) |
| `hi gateway` | Message-channel gateway; `--check` for a connection preflight |
| `hi gateway <action>` | `setup` / `start` / `stop` / `restart` / `status` / `reload` |
| `hi session <sub>` | Sessions: `list` / `show` / `export` / `compressions` / `compression-show` / `purge` |
| `hi memory <sub>` | Knot memory: `list` / `show` / `add` / `forget` / `reinforce` / `extract` |

> Append `--help` to any command for full flags, e.g. `hi gateway --help`, `hi session show --help`.

## API Server

`hi gateway` listens on `127.0.0.1:9527` by default (`[channels.http]`; set `enabled = false` to disable). The `{id}` in the URL maps to session `http:{id}` for frontends and scripts.

```bash
hi gateway    # logs the Bearer token (or see ~/.hi/hi.toml [channels.http].token)
```

| Mode | Request |
|------|---------|
| **Streaming** (SSE, default) | `POST /v1/sessions/{id}/turns`, body `{"message":"..."}` |
| **Non-streaming** (JSON) | Same, with `Accept: application/json`; response `{ "events", "reply" }` |

```bash
# Streaming
curl -sN -H "Authorization: Bearer <token>" -H "Content-Type: application/json" \
  -d '{"message":"hello"}' http://127.0.0.1:9527/v1/sessions/alice/turns

# Non-streaming
curl -s -H "Authorization: Bearer <token>" -H "Accept: application/json" \
  -H "Content-Type: application/json" \
  -d '{"message":"hello"}' http://127.0.0.1:9527/v1/sessions/alice/turns
```

Also: `GET /healthz`, `GET /v1/info`, `GET /v1/sessions`, `POST /v1/sessions/{id}/approvals` (bash approval). Details: [HTTP Gateway API](docs/guides/http-gateway-api.md).

## Memory (Knot system)

Session transcripts and long-term memory are **layered**:

| Layer | Stores | Notes |
|-------|--------|-------|
| **Bamboo slips** (`messages`) | Full conversation text | Append-only; compression affects LLM context only, full history exportable |
| **Knots** (`knots`) | Distilled atomic facts | Shared across TUI / chat / gateway; one fact per knot |

- **Lightweight** — SQLite + types + keywords, no vector DB
- **Slow write, fast read** — LLM extraction after turns or compression; SQL-only injection at read time
- **Forgetting** — `clarity` decay; manual add / reinforce / delete supported
- **On-demand retrieval** — `memory_search` for todos and decisions; system prompt injects preferences and fact baseline only

```bash
hi memory list
hi memory add "Prefers Simplified Chinese" --kind preference --confirmed
hi memory extract
```

Design: [Knot memory](docs/design/2026-06-04-knot-memory-design.md)

## Locale

- No `[locale]` in config: follows system `LANG` / `LC_*` on each start (`zh*` → Chinese, else English)
- Fixed language: `[locale] lang = "zh"` or `"en"` in `~/.hi/hi.toml`
- Override: `HI_LOCALE=zh|en`

> Messaging channels, Codex, WeChat iLink, etc. depend on third-party terms; ensure compliance before use.

## License

[MIT](LICENSE)
