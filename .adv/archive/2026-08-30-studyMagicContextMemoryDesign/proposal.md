## Cross-Project Origin

This change was created as a follow-up from **toolbox**.

| Field | Value |
|-------|-------|
| Source project | toolbox |
| Source path | `/home/jon/toolbox` |

> **Note:** The originating project should be consulted for context on why this change is needed.


## Why this exists

While researching the OpenCode plugin ecosystem from the `toolbox` project (2026-08-29), one adjacent system turned out to be a close design analogue to `episode`: **`cortexkit/magic-context`** (npm `@cortexkit/opencode-magic-context`). It solves durable cross-session memory for coding agents, the same problem `episode` solves, but with materially different architectural choices.

It is MIT licensed, so its approaches are legally reusable with attribution.

This change exists to capture that research while it is fresh and to route the transferable ideas into the changes they actually affect. It is a research/decision change, not an implementation commitment.

- Repo: https://github.com/cortexkit/magic-context
- npm: `@cortexkit/opencode-magic-context` v0.38.0 (2026-08-17), ~4.3K weekly downloads
- Created 2026-03-26, author `ualtinok`. License MIT.
- Source evidence below cites commit `a32a839`.

## Side-by-side

| Dimension | episode | magic-context |
|---|---|---|
| Language | Rust (edition 2024) | TypeScript, with an optional Rust transform twin (`crates/`, `rust-mode-transform.ts`) |
| Store | PostgreSQL + pgvector, HNSW cosine, `vector(1024)` | Single SQLite file, `bun:sqlite` / `node:sqlite`, **no native module** |
| Store location | Docker compose service (`docker compose up -d`) | `~/.local/share/cortexkit/magic-context/context.db` |
| Embeddings | Local `fastembed`, BGE-large, 1024d | Local `Xenova/all-MiniLM-L6-v2`, 384d, ~90 MB one-time |
| Scope key | Project root; product scoping in flight | `project_path` = resolved git root, plus a `harness` column |
| Ingestion | Periodic reconcile of ADV wisdom/reflection files | Hidden background subagents doing LLM summarization |
| Surface | MCP stdio server (`recall`/`remember`/`forget`/`stats`) | OpenCode plugin hooks + slash commands |
| Deployment cost | Requires a running Postgres | Zero external services |

## Transferable ideas, ranked

### 1. SQLite with no native module — feeds `decideStorageLanguageDirection`

The single highest-value input. magic-context stores everything in one SQLite file through `bun:sqlite`/`node:sqlite` and ships **no native module**. `episode` currently requires the operator to run `docker compose up -d` before it works at all, which is the main adoption friction — it is why `episode` sits declared-but-disabled in the toolbox MCP inventory today.

This is not a straight swap. `episode` depends on pgvector's HNSW cosine index over `vector(1024)`; SQLite has no native equivalent, so the honest comparison is against `sqlite-vec` or brute-force cosine over a bounded row count. Worth measuring: at realistic memory counts (thousands, not millions), brute-force cosine in Rust may be entirely adequate, which would remove the Postgres dependency without losing recall quality.

**Action:** feed this into `decideStorageLanguageDirection` as a third option alongside whatever is already under consideration. Do not decide it here.

### 2. Stable project identity across worktrees, clones, and forks — feeds `addProductScopingRecall`

magic-context keys project data on the **resolved git root**, and its docs claim the identity survives worktrees, clones, and forks. That matters directly for this machine: ADV creates per-change worktrees under `$ADV_WORKTREE_HOME`, so a naive `cwd`-based key would fragment one project's memory across every worktree it ever had.

Worth confirming how they resolve it (`git rev-parse --git-common-dir` is the likely mechanism, since that is what collapses a worktree to its parent) and whether `episode` currently has the same fragmentation bug.

**Action:** check `episode`'s current scope-key derivation against the worktree case. If it fragments, that is a defect worth its own change.

### 3. Tiered config with unsafe-field stripping — a security pattern

Project-tier config in magic-context is stripped of unsafe fields before it is honored: a project cannot set the hidden-agent models, cannot toggle compaction, and cannot supply profile content. The stated reason is that a project must not be able to reprogram cost or disable the safety layer.

This generalizes to any system that reads project-local config. If `episode` ever grows per-project configuration, adopt the same split: a trusted user tier and an untrusted project tier with an explicit allowlist. Untrusted input normalized at the boundary is P33.

### 4. Background consolidation as a first-class role

magic-context runs three named background roles as hidden child sessions: a **historian** (summarizes old history into compartments), a **dreamer** (overnight memory maintenance, ~11 manifest tasks), and a **sidekick** (retrieval augmentation). `episode` currently does a periodic reconcile of ADV wisdom and reflection files, which is ingestion, not consolidation.

The "dreamer" idea is the interesting one for memory quality: an offline pass that merges duplicates, decays stale entries, and promotes repeatedly-confirmed memories. That is adjacent to `addPromotionStateMemories`, which already contemplates promotion state.

**Caveat worth recording:** their historian falls back to the **active session model** on primary failure. On a metered subscription that silently bills the user's scarce quota. Any consolidation pass in `episode` must pin its model explicitly and fail closed rather than fall back to the caller's model.

### 5. Embedding size tradeoff

They use a 384d model at ~90 MB; `episode` uses BGE-large at 1024d. Theirs is cheaper to store and faster to compare; `episode`'s is more accurate. Not a defect on either side, but if the SQLite direction is pursued, the smaller vector materially changes what brute-force cosine costs.

### 6. Cross-harness scoping

A `harness` column lets one store serve OpenCode, Pi, and OMP simultaneously. `episode` is ADV/OpenCode-specific by design. Recording as a known generalization path, not a recommendation.

## Explicitly NOT recommended for adoption

magic-context's core context-management mechanism is **prompt-prefix rewriting** via the `experimental.chat.messages.transform` hook — `messages-transform.ts:51-53` performs `output.messages.splice(0, len, ...next)`, a full-array replacement. It compresses old text, strips reasoning blocks, and substitutes history with summaries.

The toolbox has a recorded ADR (0013) establishing that rewriting already-cached prompt-prefix content forces Anthropic to re-bill the entire prefix as `cache_creation` — the token class Claude Max meters — measured at roughly 15 quota tokens burned per context token saved.

To its credit, magic-context is the only context plugin found that models provider cache TTL in its scheduler at all, with a per-model `cache_ttl` map (`30s|5m|1h|never`). But its default is `"5m"`, which matches Claude **Pro**; Max is a 1-hour window. It also forces `compaction.auto: false`.

None of this applies to `episode` directly — `episode` is an MCP server and never touches the prompt array. It is recorded here so nobody later mistakes prefix rewriting for a validated technique worth copying.

## Acceptance for this change

This change is complete when the ideas above are routed, not when anything is built:

1. Idea 1 is attached to `decideStorageLanguageDirection` as an evaluated option with a measurement plan.
2. Idea 2 is checked against `episode`'s current scope-key derivation, and a defect change is opened if it fragments across worktrees.
3. Idea 4's caveat (pin the model, never fall back to the caller's) is recorded wherever consolidation is specified.
4. Ideas 3, 5, and 6 are recorded as known options with no action required.

## Provenance

Researched from `toolbox` on 2026-08-29 via `adv-researcher` against the repo at commit `a32a839`, the npm registry metadata, and the project's `CONFIGURATION.md` and README. One item is **UNVERIFIED**: `ARCHITECTURE.md` (58 KB, repo root) documents the internal transform pass taxonomy and byte-identical replay guarantees; the fetch was dropped mid-session. Read it before acting on idea 1 or 4.
