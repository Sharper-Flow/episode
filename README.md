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
| Embeddings | Local `fastembed` (BGE-large, 1024d) — English-only; the only backend in v0; `EPISODE_EMBED_BACKEND` accepts only `local` |
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
automation and versioning convention. Operational context, capability specs,
and capacity evidence for pool/ingestion bounds are maintained in
[`project.md`](project.md) and [`docs/specs/`](docs/specs/).

## Configuration

Copy `.env.example` to `.env` and adjust for your environment. Required and
behavioral settings are validated at startup; invalid values fail with the
offending variable name and expected format. `EPISODE_LOG_LEVEL` intentionally
defaults to `INFO` when absent, empty, or invalid so logging cannot prevent
startup. Key variables:

| Variable | Purpose |
|---|---|
| `EPISODE_DATABASE_URL` | Postgres + pgvector connection |
| `EPISODE_DB_POOL_SIZE` | Connection pool size (default: 10) |
| `EPISODE_INGEST_INTERVAL_SECS` | Seconds between ADV ingestion passes (default: 60) |
| `EPISODE_EMBED_BACKEND` | Only `local` is supported in v0; other values are rejected at startup |
| `EPISODE_LOG_LEVEL` | `trace`/`debug`/`info`/`warn`/`error`; invalid or absent values default to `INFO` |
| `EPISODE_PROJECT_ROOTS` | Comma-separated `namespace=path` pairs for ADV wisdom/reflection ingestion |

See `.env.example` for full descriptions and defaults.

## Development

Requires Rust, Docker, and the bundled pgvector Postgres.

```bash
docker compose up -d          # dev Postgres + pgvector on :5434
cp .env.example .env          # then edit as needed
cargo run                     # serves MCP over stdio
```

Run the unit suite (no external services):

```bash
cargo test --locked
```

Run the end-to-end recall test (needs the dev DB; downloads the model once):

```bash
cargo test --test recall_it -- --ignored --nocapture
```

## Deploy

`scripts/deploy.sh` builds a release binary to `~/.local/bin/episode`, which
Vision spawns and proxies (one instance, many agent sessions).

## Language support and scope

Episode v0 is intentionally English-only. It uses the local fastembed
BGE-large model (1024d), and no multilingual evaluation or embedding-model
replacement is in scope. Non-English content will still be embedded and stored,
but recall quality is only guaranteed for English input.

## Relationship to prior resilience work

The archived change `.adv/archive/2026-07-10-improveEpisodeResilience/` already
delivered bounded pool acquisition, bounded batch processing, and manual-memory
deletion restrictions. Those protections remain in force and are not duplicated
here. This change extends Episode's operational contracts — schema verification,
logging, release automation, and documentation — without altering the runtime
protections established by that archive.

## Project context and specs

For ADV project context, conventions, and the canonical capability specs, see
[`project.md`](project.md). Human-readable specs are in [`docs/specs/`](docs/specs/);
authoritative branch-local specs are in [`.adv/specs/`](.adv/specs/).

## License

MIT © Sharper-Flow
