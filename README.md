# episode

Persistent **decision memory** for AI coding agents, served over the [Model Context Protocol (MCP)](https://modelcontextprotocol.io).

`episode` gives a fleet of OpenCode agents a durable, retrievable memory of *why* things were decided — gotchas, conventions, failed approaches, and post-change reflections — so sessions stop starting blind. It ingests [Advance (ADV)](https://github.com/Sharper-Flow) wisdom and reflections, embeds them, and serves semantic recall from a pgvector store. One instance serves many concurrent agents.

> **Scope on purpose:** episode is the *decision* memory layer. It deliberately does **not** re-index source code — that job belongs to a code-search tool (e.g. `lgrep`). Recall bridges to code search; it doesn't duplicate it.

## What it is

| Concern | Choice |
|---|---|
| Language | Rust (fast, one instance serves ~20 concurrent agent sessions) |
| Protocol | MCP over stdio (`rmcp`), proxied by [Vision](https://github.com/Sharper-Flow/Vision-MCP-Manager) |
| Store | PostgreSQL + `pgvector` (HNSW, cosine, `vector(1024)`) |
| Embeddings | Local `fastembed` by default; `voyage-4-lite` optional tier |
| Ingestion | Reads each project's `.adv/wisdom.jsonl` + `.adv/reflections.jsonl` |
| Namespacing | Per-project namespace + a shared `global` namespace |

## Tools (MCP)

| Tool | Purpose |
|---|---|
| `recall` | Semantic search over memories (namespace-filtered, top-k) |
| `remember` | Write a memory directly (manual, `global` or a project namespace) |
| `forget` | Soft-remove a memory by id |
| `stats` | Counts by namespace / source |

## Architecture

```
OpenCode agents ──stdio──▶ Vision proxy ──▶ episode (rmcp)
                                              │
                    ┌─────────────────────────┼───────────────────────┐
                    ▼                          ▼                       ▼
             recall / remember          embedder                 ingestion
             (MCP tools)          (fastembed | voyage)     (.adv/*.jsonl watcher)
                    │                          │                       │
                    └──────────────▶ Postgres + pgvector ◀─────────────┘
                                     (HNSW cosine, vector(1024))
```

**Ingestion note:** project `.adv/wisdom.jsonl` is compacted to a rolling window by ADV, so episode's store is the durable *superset* — it ingests entries before they age out and never compacts them.

## Status

🚧 Early. Dependency foundation + schema are in place; the MCP server, store, embeddings, and ingestion land phase by phase (see the build plan in project notes).

## Development

Requires Rust, Docker, and a pgvector-capable Postgres.

```bash
docker compose up -d          # dev Postgres + pgvector on :5433
cp .env.example .env          # then edit as needed
cargo run
```

## License

MIT © Sharper-Flow
