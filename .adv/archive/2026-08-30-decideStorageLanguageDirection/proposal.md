# Storage and language direction

## Problem

Episode is Rust + Postgres + pgvector. Concord, its primary orchestration context, is Go and keeps a single local SQLite authority under CD-0002. Before implementing the Tier 1–3 structured-memory work, we need a recorded direction on:

1. Stay Rust + Postgres for the foreseeable work?
2. Rewrite to Go + SQLite now to align with Concord?
3. Condition the rewrite on a trigger?

This decision gates the implementation language of every downstream change in the epic.

## Analysis

### What Concord actually requires of episode

The first question is whether Concord's storage law reaches episode at all. It does not.

CD-0002 (`docs/decisions/CD-0002-concord-state-authority.md:19-31`) states that SQLite is **Concord's** sole durable authority and that "no second store holds authoritative state." That prohibition is scoped to Concord's own state. It is silent on satellite services, and CD-0002:104-109 names DBOS on Postgres as a "strong buy fallback if SQLite ever proves insufficient" — for Concord itself. Nothing in it forbids a Postgres-backed external service.

Concord's Go direction is likewise scoped. `docs/core-architecture.md:40-60` places Go at "the Concord core," and `:85-91` keeps Rust explicitly available. There is no documented language or storage requirement for companion services, and Concord's Go code contains zero references to episode.

Concord treats episode as one of three independent general-purpose tools. `docs/vertical-integration.md:64` states plainly: "Tools stay independent; Concord orchestrates product context." The promotion seam specifically crosses a manifest-path + sha256 boundary (`:130-158`), holds a `target` that is opaque to episode, and records no runtime event. That seam is language- and storage-agnostic by construction. It is the promotion boundary, not the whole integration surface, so it should not be cited as though it governed every interaction.

**What Concord has decided.** The episode ownership question is clarification **C20**, split out of C8 by concord#101 precisely so episode's trigger could fire without implying a direction change for lgrep and vision. Concord's operator approved a direction on 2026-08-30, recorded in concord#46: **episode stays external, optional, and Product-scoped when configured**, with the product-scoping probe demoted to a reopen trigger rather than an acceptance prerequisite.

That approval was recorded in the issue before its documentation deliverables ran, so Concord's accepted documents lagged behind it for a period and still described C20 as open. Concord PR #596 executes those deliverables and closes the gap.

**This decision does not depend on that approval.** The argument above rests on scope: CD-0002's authority rule and the Go direction do not reach satellite services. That holds regardless of how C20 resolves, which is why it is the ground this decision stands on. Concord's approval corroborates the boundary; it does not create it.

So there is no architectural divergence to resolve. There are two systems with a documented boundary between them, and an open ownership question that neither side has closed.

### Case for a Go + SQLite rewrite now
- One language across Concord and episode would reduce integration friction.
- sqlite-vec is reachable from Go by at least three paths, two of them cgo-free: `modernc.org/sqlite` ships sqlite-vec transpiled in its `vec/` package (v1.57.0, auto-registering `vec0` on blank import), `ncruces/go-sqlite3` offers a WASM path, and `mattn/go-sqlite3` enables extension loading by default. Dropping Postgres would remove a daemon from the deployment.
- Porting is cheaper earlier than later, though less cheap than it was: the codebase is now 3225 lines across 9 source files and 2 migrations, up from roughly 1500 lines, 6 files, and 1 migration when this question was first framed. Tier 1–3 will widen that gap further.

### Case against — the recommendation
- **Concord imposes no such requirement.** CD-0002's authority rule and the Go direction are both scoped to Concord's own core. A rewrite would buy alignment that the architecture does not ask for.
- **pgvector is materially more mature than sqlite-vec for this workload.** sqlite-vec's current stable release is v0.1.9 (2026-03-31), with v0.1.10 still in alpha; it is pre-v1 and documents that it expects breaking changes. Its stable KNN is exhaustive, and ANN work remains experimental behind compile flags. pgvector ships steady releases through 0.8.6 with HNSW and iterative scans. Both projects are 0.x, so the gap is cadence, adoption, and ANN depth rather than a version badge.
- **`vec0` carries layout-inherent write amplification.** Space is reclaimed only in the latest chunk, there is no vacuum, and a production report describes the database growing monotonically with every re-embedding. Episode's ingest is append-mostly, so exposure is workload-dependent rather than disqualifying — but it is unmeasured.
- **The metadata and product-scoping gaps are language-independent.** They are data-model and API gaps. Moving Rust to Go does not close them faster; it resets verified-working maturity to zero while leaving the same gaps open.
- **C20's lean is product-scoping first**, deferring ownership until measured need. A rewrite is a step toward owning, taken before any evidence supports it.
- **The probe has to add metadata and product features to something.** Doing that in Rust lets us measure whether orchestration suffices. Rewriting first assumes the answer.

### Alternatives considered

**Go + SQLite (full rewrite).** Rejected on the grounds above.

**Rust + SQLite — keep the language, drop the server.** This is the strongest alternative and it deserves an explicit rejection rather than silence. It answers the deployment objection at far below the cost of a language rewrite: `store.rs`, the migrations, and config change; `rmcp`, `fastembed`, and the ingest path stay. It is capability-feasible today, since `rusqlite` supports extension loading through `LoadExtensionGuard` when the non-default `load_extension` crate feature is enabled.

It is rejected for now because it loses on retrieval and on filtering:
- `vec0`'s stable KNN is exhaustive where pgvector offers HNSW.
- SQLite has no JSONB + GIN equivalent. `vec0` metadata columns are documented as slower on scans and inefficient past 12-character strings, which directly weakens the recall filters that spec 0010 already delivers.
- The engine is pre-v1 with the write-amplification characteristic described above.
- Postgres is already deployed and verified end to end, and the pain this would relieve is unmeasured.

Recorded as a real option, rejected on evidence, and available for reconsideration at the checkpoint below.

### The decisive tension

Postgres against SQLite is the real fork, and it sits downstream of the C20 ownership question, which sits downstream of the product-scoping probe. We should not rewrite until C20 resolves toward owning, and C20 should not resolve toward owning until product-scoping is proven insufficient.

### A premise this decision does not rest on

An earlier draft argued that Concord was pre-runtime and that aligning with it was therefore speculative. That premise is false and was false when written. Concord has a working runtime: `cmd/concord/main.go` routes launcher, session, and JSON commands across a full operator surface; `internal/store` holds a SQLite `domain_events` log with projections; `internal/agent/runtime.go` dispatches; a Bubble Tea TUI ships. `docs/floor-readiness.v1.json` records 40 items satisfied, 1 out of scope, 0 outstanding. Concord was v0.10.1 when this question was framed and is v4.9.0 today. The README says "pre-replacement-readiness," which the earlier draft misread as "pre-runtime."

This is recorded because the refuted premise is the one a reader is most likely to reconstruct independently, notice is false, and conclude the decision is stale. The decision does not depend on it. Concord's maturity is an argument for a stable documented boundary, not against one.

## Recommendation

**Stay Rust + Postgres for all Tier 1–3 work.** Treat the storage and language rewrite as a conditioned future rather than planned work.

- **Trigger source:** the `addProductScopingRecall` probe outcome — does product-scoping suffice for real multi-project Product work?
- **Concord-side tracker:** [concord#46](https://github.com/Sharper-Flow/concord/issues/46) Item 1 drives the C20 direction update.
- **Evidence bar:** measured pain that product-scoping cannot resolve. Not a vibe, not a calendar date.

**What the trigger actually fires.** An insufficient probe outcome does **not** automatically fire a Go + SQLite rewrite. It reopens the C20 ownership question. Only if C20 then resolves toward Concord owning the memory territory does the storage and language choice come back into scope, and at that point Rust + SQLite and Go + SQLite are both live options to be re-costed against the measurement plan below. Recording the trigger as a direct rewrite switch overstates what the evidence would establish.

Record the rewrite as a named conditional follow-up, not an implicit maybe.

## Trigger viability — the weakest part of this decision

AC3 requires a conditioned follow-up rather than a dangling option. That test is not currently passed. The probe cannot produce evidence today, for three independent reasons, and one of them is time-limited:

1. **Episode is disabled on the host.** Its MCP entry exists but is switched off (concord#46, 2026-08-14), so no usage evidence accumulates. No cause is recorded; deployment friction is neither confirmed nor excluded. *Action: re-enable episode, and record why it was off.*
2. **The probe does not exist.** `addProductScopingRecall` is not implemented in this repository, and spec 0010 explicitly defers automatic product scoping and the probe. Concord's own 2026-08-30 trigger-state check records the same finding. *Action: implement the probe change.*
3. **The evidence window is finite.** Episode's ingestion source is predecessor wisdom and reflection state, which retires when the predecessor retires. This is recorded in C20's rationale and in the background of concord#101, the (now closed) issue that split C8 into C20. *Action: re-point ingestion before predecessor retirement.*

A condition that cannot be evaluated is a dangling option by AC3's own letter. If the window closes before the probe runs, this decision silently becomes "never revisit," which nobody chose.

**Option-decay checkpoint.** Porting cost roughly doubled in 19 days, from ~1500 to 3225 lines, and Tier 1–3 will push it further. Deferring dominates only while the option holds value. Re-decide this change if either holds:
- predecessor retirement arrives with no probe evidence, or
- the codebase passes a size threshold at which the port stops being tractable.

At that checkpoint, reconsider Rust + SQLite alongside the Go + SQLite option, since it captures the deployment benefit without a language rewrite.

## Decision outcome

This change produces a decision record capturing:
- The direction — Rust + Postgres now
- The concrete trigger — the `addProductScopingRecall` probe proving product-scoping insufficient — and what it actually fires, which is a C20 reopen rather than a rewrite
- The trigger's current blockages, their unblocking actions, and the option-decay checkpoint
- The Concord-side tracker — concord#46 Item 1 for the C20 direction update
- The alternatives considered, including Rust + SQLite, and why each is rejected
- The evidence bar for revisiting

## Acceptance criteria

- AC1: A decision record exists documenting the direction, the concrete trigger, alternatives, and revisit conditions.
- AC2: All downstream epic changes reference this decision as their language assumption.
- AC3: The Go+SQLite rewrite is recorded as a conditioned named follow-up bound to the `addProductScopingRecall` probe outcome, not a dangling option.

## Epic context

Member of Epic `shapeEpisodeStructuredMemory` (advisory order 1 of 6). Sequenced first because it gates the implementation language of every downstream change. Bidirectionally linked to concord#46 for the C20 question, the probe, and the outcome.

## External evidence — magic-context

> Routed from change `studyMagicContextMemoryDesign`. Source: [`cortexkit/magic-context`](https://github.com/cortexkit/magic-context) (MIT), read at HEAD `a54f9c0`. This section is **corroborating evidence only**. It does not re-open the decision above, and it adds no new option. The recommendation, the trigger, and AC1–AC3 stand unchanged.

A close design analogue exists and made the opposite storage choice. It is worth recording what their choice actually buys, because the naive reading — "they removed the Postgres dependency, so we could too" — does not survive contact with their own design documentation.

### What they built

- Exact brute-force fp32 cosine in application code. The similarity engine (`memory/cosine-similarity.ts`) is a dot product and two norms, and nothing else. There is **no ANN index**.
- Vectors are plain SQLite BLOBs in a side table (`storage-db.ts`):

```sql
CREATE TABLE IF NOT EXISTS memory_embeddings (
  memory_id INTEGER NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
  embedding BLOB NOT NULL,
  model_id TEXT NOT NULL,
  PRIMARY KEY(memory_id, model_id)
);
```

  The composite primary key is worth noting on its own: they store one row **per memory per embedding model**, so a model change adds vectors alongside the old ones rather than destroying them. If episode ever changes embedding model, that is the shape that makes the migration survivable.

- Their stated operating range is hundreds to roughly 1000 memories per project at 384 dimensions. `MEMORY-DESIGN.md` reports ~79 memories in 0.1s cached, ~10 ms per query, and 1000 memories ≈ 1.5 MB.

### Why this corroborates staying on pgvector

Their own design document names the exit condition: *"At 1000+ memories, if full-scan latency is noticeable, add ANN index or per-project shard cache."*

That is the escape hatch episode already occupies. pgvector HNSW is not an over-engineered choice relative to their design — it is the thing their design defers until it needs it.

Their ~10 ms figure is a **full-scan** number at 384 dimensions. episode does not full-scan; HNSW is an approximate-nearest-neighbour index, so the two numbers measure different operations and cannot be compared directly. The figure becomes relevant only in the counterfactual where episode abandoned the index for brute force — and there the cost would be worse than theirs by more than the row count alone suggests, because a 1024-dimension fp32 vector is 4096 bytes against their 1536, roughly 2.7x the bytes per row scanned.

### On sqlite-vec — read this before citing them

magic-context **rejected** sqlite-vec, and it would be easy to cite that rejection as counter-evidence against sqlite-vec generally. It is not. Their stated reason has two parts, and the parts transfer differently:

1. **"`bun:sqlite` can't load extensions" — runtime-specific, does NOT transfer.** This is a property of their JavaScript runtime, not of sqlite-vec. Rust's `rusqlite` supports extension loading through `load_extension_enable` and `LoadExtensionGuard`, provided the non-default `load_extension` crate feature is enabled. In Go, `mattn/go-sqlite3` enables extension loading by default, and two cgo-free paths exist as well: `modernc.org/sqlite` ships sqlite-vec transpiled in its `vec/` package, and `ncruces/go-sqlite3` provides a WASM route.
2. **"write-amplification" — DOES transfer.** This is a property of sqlite-vec's `vec0` storage layout, not of Bun. It remains live, workload-dependent evidence against sqlite-vec in a Rust or Go evaluation, and it is not answered by anything above.

So magic-context is not evidence that sqlite-vec is unavailable outside Bun, and availability is not the reason this decision rejects it. The rejection rests on the boundary facts and on sqlite-vec's maturity and retrieval characteristics. One substantive objection — write amplification — is left standing for a future evaluation to measure rather than assume.

### Measurement plan, for the trigger — not for now

If C20 ever reopens toward owning under the trigger above, or the option-decay checkpoint reopens this, the storage comparison to run is:

- brute-force fp32 cosine in Rust,
- sqlite-vec `vec0`,
- pgvector HNSW,

measured at realistic episode row counts **at 1024 dimensions**, with `vec0` write-amplification measured under the real ingest cadence rather than assumed from the citation above.

This is recorded so the trigger has a concrete first action. It is not a reason to fire the trigger.
