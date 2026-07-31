# episode — Project Context

> Read by the ADV `adv_project_context` tool. For contributor-facing quick reference, see [`README.md`](README.md).

## What This Is

`episode` is a Rust stdio MCP server that gives OpenCode agent fleets durable, semantic **decision memory**. It stores gotchas, conventions, failed approaches, and post-change reflections in PostgreSQL + pgvector and serves `recall` / `remember` / `forget` / `stats` tools over MCP.

Scope on purpose: episode indexes decision memories, not source code. Source-code recall is the job of a code-search tool (e.g. `lgrep`).

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust (edition 2024) |
| Protocol | MCP over stdio (`rmcp`) |
| Store | PostgreSQL + pgvector (HNSW cosine, `vector(1024)`) |
| Embeddings | Local `fastembed` (BGE-large, 1024d) — English-only in v0 |
| Ingestion | Periodic reconcile of each ingested project root's ADV wisdom and reflection files |
| CI / Release | GitHub Actions (`.github/workflows/ci.yml`, `.github/workflows/release.yml`) |

## Key Directories

```
src/                 # Rust source
migrations/          # SQLx migrations (single init migration)
tests/               # Integration tests (DB-backed tests are #[ignored])
scripts/             # Local deployment helpers
docs/                # Human-facing documentation
docs/specs/          # Human-readable capability specs
.adv/specs/          # ADV capability specs — branch-local laws
.adv/archive/        # Immutable completed-change archives
```

## Development Commands

```bash
# 1. Start the dev database
docker compose up -d

# 2. Configure
cp .env.example .env   # edit as needed

# 3. Run / test
cargo run              # serves MCP over stdio
cargo test --locked    # unit suite (no external services)

# End-to-end recall test (needs Postgres + one-time model download)
cargo test --test recall_it -- --ignored --nocapture

# Release profile (what CI and scripts/deploy.sh use)
cargo build --release --locked
```

Local deploy: `scripts/deploy.sh` builds and installs the release binary to `~/.local/bin/episode`.

## Configuration

Copy `.env.example` to `.env`. All variables are validated at startup; invalid values fail with the offending variable name and expected format. Key variables:

| Variable | Purpose |
|---|---|
| `EPISODE_DATABASE_URL` | Postgres + pgvector connection |
| `EPISODE_DB_POOL_SIZE` | Connection pool size (default: 10) |
| `EPISODE_INGEST_INTERVAL_SECS` | Seconds between ADV ingestion passes (default: 60) |
| `EPISODE_EMBED_BACKEND` | Only `local` is supported in v0; other values are rejected |
| `EPISODE_LOG_LEVEL` | `trace/debug/info/warn/error`; invalid/absent defaults to `INFO` |
| `EPISODE_PROJECT_ROOTS` | Comma-separated `namespace=path` pairs to ingest |

## Operational Boundaries

- **English-only embeddings.** v0 uses the local fastembed BGE model and supports English input only. Multilingual embedding support and embedding-model replacement are explicitly out of scope.
- **Archive protection.** `.adv/archive/2026-07-10-improveEpisodeResilience/` is the completed resilience change. Its protections — bounded pool acquisition, bounded batch processing, and deletion source/namespace restriction — remain in force. This change extends operational contracts without duplicating them.
- **CI-gated releases.** Releases are published only after the `CI` workflow succeeds on `main`. See [`docs/release.md`](docs/release.md).

## Specs

Capability specs live in `.adv/specs/` (authoritative, machine-readable context) and `docs/specs/` (human-readable). Specs are git-tracked and branch-local; they are not copied across worktrees until merged.
