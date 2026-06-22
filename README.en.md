# hi

Rust personal AI assistant: **single binary** · local TUI · optional IM gateway · one shared Agent core

> **Personal learning project** — for practicing Rust, agent runtimes, TUI, and messaging-channel integration. Features evolve continuously; **not production-ready**. Feedback welcome; do not rely on it for critical workloads.

## Why hi

- **Tiny & instant** — release binary ~10 MB, no extra runtime; cold start straight into `hi chat` / `hi tui`
- **Lightweight** — four core tools + optional memory, no MCP/Skills stack; one config file
- **One core, many surfaces** — TUI, terminal chat, WeCom / Feishu / WeChat iLink share the same Agent; sessions isolated per channel
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

GitHub Release publishes `@zggg/hi` to GitHub Packages. For `@i99/hi` on npmjs, run `./scripts/build-dist.sh` and publish locally.

Docs: [Install](docs/guides/install.md) · [Architecture](ARCHITECTURE.md) · [Security](docs/SECURITY.md)

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
