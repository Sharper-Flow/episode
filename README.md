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
| Embeddings | Local `fastembed` (BGE-large, 1024d) — the only backend in v0; `EPISODE_EMBED_BACKEND` accepts only `local` |
| Ingestion | Periodic reconcile of each project's `.adv/wisdom.jsonl` + `.adv/reflections.jsonl`, embedded in bounded batches |
| Namespacing | Per-project namespace + a shared `global` namespace |

## Tools (MCP)

| Tool | Purpose |
|---|---|
| `recall` | Semantic search over memories (namespace-filtered, top-k) |
| `remember` | Write a memory directly (manual, `global` or a project namespace) |
| `forget` | Restricted hard deletion of a `manual` memory by id + namespace (ingested memories are never affected) |
| `stats` | Counts by namespace / source |

## Architecture

```
OpenCode agents ──stdio──▶ Vision proxy ──▶ episode (rmcp)
                                              │
                    ┌─────────────────────────┼───────────────────────┐
                    ▼                          ▼                       ▼
             recall / remember          embedder                 ingestion
             (MCP tools)          (local fastembed)     (periodic .adv reconcile)
                    │                          │                       │
                    └──────────────▶ Postgres + pgvector ◀─────────────┘
                                     (HNSW cosine, vector(1024))
```

**Ingestion note:** project `.adv/wisdom.jsonl` is compacted to a rolling window by ADV, so episode's store is the durable *superset* — it ingests entries before they age out and never compacts them.

## Status

**v0 working.** MCP server (`recall`/`remember`/`forget`/`stats`), pgvector store
(HNSW cosine), local fastembed embeddings (BGE-large, 1024d), and ADV
wisdom/reflection ingestion are implemented and verified end-to-end (real embed →
store → semantic recall ranks correctly). Runs under Vision as a shared server.
The embedding backend is local-only in v0: `EPISODE_EMBED_BACKEND` accepts only
`local` (a `voyage` value is rejected at startup), and ingestion is a periodic
reconcile loop (see `EPISODE_INGEST_INTERVAL_SECS`), not a file watcher.

Not yet implemented: a Voyage embedding tier, file-watch ingestion.
Releases are CI-gated; see [`docs/release.md`](docs/release.md) for the
automation and versioning convention.

## Development

Requires Rust, Docker, and the bundled pgvector Postgres.

```bash
docker compose up -d          # dev Postgres + pgvector on :5434
cp .env.example .env          # then edit as needed
cargo run                     # serves MCP over stdio
```

Run the end-to-end recall test (needs the dev DB; downloads the model once):

```bash
cargo test --test recall_it -- --ignored --nocapture
```

## Deploy

`scripts/deploy.sh` builds a release binary to `~/.local/bin/episode`, which
Vision spawns and proxies (one instance, many agent sessions).

## License

MIT © Sharper-Flow
