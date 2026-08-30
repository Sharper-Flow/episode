## Design — routing plan

This change writes to sibling ADV change artifacts. It writes no `episode` source. Each routing action below names its target change, the insertion, and the evidence that justifies it.

### Mechanism

Each target receives an appended `## External evidence — magic-context` section in its proposal, via `adv_change_update`. Appending rather than rewriting preserves each sibling's authored argument and keeps the provenance of the addition legible. No sibling's acceptance criteria, scope, or recommendation is edited: this change supplies evidence, it does not re-decide sibling decisions.

Source basis for every citation: `cortexkit/magic-context`, MIT, read at HEAD `a54f9c0` with a cross-fetch diff against `a32a839`. The Memory, Dreamer, and Historian sections are substantively identical between the two commits. Every load-bearing claim below was independently re-verified against the repo at `a54f9c0` during design validation.

---

### R1 → `decideStorageLanguageDirection`

**Disposition: corroborates the existing recommendation. Adds no new option.**

The sibling already recommends staying Rust + Postgres and conditioning a Go + SQLite rewrite on the `addProductScopingRecall` probe. The proposal for this change asked for SQLite to be added "as a third option". Discovery refutes that framing, so the routing is inverted: magic-context is evidence *for* the existing recommendation.

Content to append:

- magic-context runs exact brute-force fp32 cosine over SQLite BLOBs (`memory/cosine-similarity.ts`, 19 lines: dot product plus two norms). No ANN index. Vectors live in a side table `memory_embeddings(memory_id INTEGER PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE, embedding BLOB NOT NULL)`, and all embeddings are loaded into memory for the scan.
- Their design target is hundreds to ~1000 memories per project at 384d. `MEMORY-DESIGN.md` states ~79 memories in 0.1s cached, ~10ms per query, and 1000 memories ≈ 1.5 MB.
- Their own escape hatch: "At 1000+ memories, if full-scan latency is noticeable, add ANN index or per-project shard cache." `episode` already runs that escape path via pgvector HNSW.
- Their throughput figures do not transfer. At 1024d fp32 an `episode` vector is 4096 bytes against their 1536, roughly 2.7x the bytes scanned per row. Both sides are full fp32 scans, so bytes-per-row is a fair cost proxy.

**Correction that must travel with the citation, scoped precisely.** The sibling's "case for" lists "sqlite-vec exists for Go" as a rewrite argument. magic-context rejected sqlite-vec for two stated reasons: "`bun:sqlite` can't load extensions + write-amplification". These transfer differently and must not be collapsed.

- The **extension-loading** reason is runtime-specific and does not transfer. Rust's `rusqlite` provides `load_extension_enable` and `LoadExtensionGuard`; Go's `mattn/go-sqlite3` enables extension loading by default. One caveat for a future Go choice: the pure-Go `modernc.org/sqlite` driver cannot load C extensions, so a Go rewrite that wanted sqlite-vec would be constrained to the cgo driver.
- The **write-amplification** reason does transfer. It is a property of sqlite-vec's `vec0` storage, not of `bun:sqlite`, and remains live workload-dependent evidence in any Rust or Go evaluation.

So magic-context is not evidence against sqlite-vec's *availability* outside Bun, but it does leave one substantive objection standing. Recording both halves separately prevents a later reader from over- or under-reading the same source.

Net effect on the decision: none. The direction and the trigger stand unchanged. AC3's conditioned follow-up remains bound to the probe outcome, not to this evidence.

**Measurement plan, recorded for the trigger.** If the `addProductScopingRecall` probe ever fires the rewrite, the storage comparison to run is: brute-force fp32 cosine in Rust, sqlite-vec `vec0`, and pgvector HNSW, measured at realistic `episode` row counts at 1024d, with write-amplification measured for `vec0` under the ingest cadence rather than assumed. This is recorded so the trigger has a concrete first action, not so it fires now.

---

### R2 → `addPromotionStateMemories`

**Disposition: evidence for an already-deferred item. Adds no scope.**

The sibling puts "auto-promotion heuristics" out of scope, keeping manual flagging first and naming heuristic promotion "a later candidate". magic-context has a working implementation of exactly that later candidate, so the routing supplies design evidence for the deferred item without pulling it forward.

Content to append:

- Promotion by repeated confirmation: candidates cluster at cosine ≥ 0.85 with 0.02 hysteresis, and promote when they span ≥ `PRIMER_MIN_SPAN_DAYS` (= 7) distinct days **and** ≥ `PRIMER_PROMOTION_THRESHOLD` (= 2) distinct occurrence keys (`primer-clustering.ts:5-8`). The two-axis threshold — distinct days *and* distinct occurrences — is the transferable part: it resists both a single chatty session and a single stale repetition.
- A second, simpler path: user-memory observations promote after ≥ 3 observations (`task-executor.ts` ~L434, `promotionThreshold ?? 3`).
- Deduplication before promotion is exact, not fuzzy: normalized content hash per category, backed by `idx_memories_project_category_hash`. Cross-category merges are structurally rejected.
- Keep-versus-discard is verified against reality rather than inferred: their `verify` task gates each memory on whether its backing file changed since `verified_at`. Partial progress persists, and refusals count as non-failing skips. `episode` ingests from ADV wisdom and reflection files, so the same file-backed verification is available to it.
- Concurrency, if a background pass is ever added: memory-mutating tasks share one conflict-domain lease keyed `memory:<project>` (`lease.ts:8`), and read-then-write runs atomically under `BEGIN IMMEDIATE`. The Postgres analogue is an advisory lock. The design point is that a consolidation pass must not race live `recall`/`remember` traffic.

**Model-pinning caveat, stated precisely.** The proposal for this change asserted that their historian "falls back to the active session model", and implied the pattern was general. It is confirmed for the historian only: `runFallbackHistorianPass` (`compartment-runner-historian.ts:506-623`) appends the live session provider/model as an absolute last resort *after* the configured chain, and `ARCHITECTURE.md` labels this "historian-only". It is **refuted** for `dreamer` and `sidekick`: both resolve through configured `fallback_models` only and throw when that list is exhausted (`resolve-fallbacks.ts`, `shared/model-resolution.ts`, `sidekick/agent.ts:29`, `model-suggestion-retry.ts`).

One trap worth recording with the citation: `resolve-fallbacks.ts`'s header comment loosely describes "the runner's session-model last resort" as if it applied to all hidden agents. Only the historian runner implements it. The code-level reading is the correct one, and a future reader consulting only that comment would draw the broader, wrong conclusion.

The caveat that lands in the sibling is therefore forward-looking and narrow: if `episode` ever adds an LLM-backed consolidation pass, pin its model explicitly and fail closed. Never fall back to the caller's session model, because on a metered subscription that silently bills the user's scarce quota. This is recorded now because the sibling's current design is manual-flagging only and has no LLM pass to constrain yet.

---

### R3 → `improveRecallQualityIngest`

**Disposition: new finding, approved at the discovery checkpoint. Recorded as a candidate sub-part, not adopted.**

The sibling covers kind/source filters (3a), recency (3b), and ingest abstraction (3c). It has no retrieval-mechanism sub-part. This finding is a candidate 3d.

Content to append:

- magic-context pairs dense cosine with an FTS5 side table `memories_fts`, kept in sync by triggers. Scores are hybrid-combined, and FTS serves as a standalone fallback when embeddings are unavailable.
- The stated reason is a documented weakness of their embedding model: weaker on exact symbols, file paths, and config keys than on prose. `MEMORY-DESIGN.md` recommends benchmarking `bge-small-en-v1.5` if symbol and path lookups stay weak.
- Relevance to `episode`: its memories are ADV wisdom and reflection entries, saturated with file paths, config keys, and symbol names. The weakness class is inherent to dense retrieval and is reduced, not removed, by `episode`'s larger BGE-large model.

**A defect in the source, recorded because it changes what transfers.** magic-context's own comment claims its tokenizer preserves `.`, `_`, `/`, and `-`. Its actual DDL is `tokenize='porter unicode61 categories "L* N* Co"'`. Per the SQLite FTS5 documentation, `L* N* Co` is the unicode61 default and is therefore a no-op, and punctuation remains a separator. Preserving those characters requires `tokenchars`, which appears nowhere in their repo. Their configuration does not do what their comment says it does.

This matters in two ways. First, their hybrid retrieval is not actually tuned for technical tokens, so their implementation is weaker evidence for the technique than their prose suggests — the *idea* transfers, their *validation* of it does not. Second, Postgres has the same trap: the default `tsvector` parsers split path and symbol tokens much as unicode61 does, so a naive `tsvector` addition in `episode` would reproduce the same gap. That makes `pg_trgm` trigram matching the more load-bearing half of the transfer, not the optional half it appeared to be before this check.

Transfer cost remains low — `tsvector`/`tsquery` and `pg_trgm` are native to Postgres, so no new dependency is required — but the tokenizer configuration is the part that must be got right, and the source cannot be copied for it.

**Constraint inherited from the sibling.** Its AC3 forbids score-blending weights without calibration evidence. A hybrid ranker is a blended score, so it falls under that bar and must not be adopted without calibration. The honest framing is that hybrid retrieval reduces the calibration problem to a single mixing weight but does not eliminate it. Recording it as a candidate respects AC3; adopting it here would violate it.

A related tool worth naming: their `embedding-baseline.ts` and `embedding-baseline-diff.ts` snapshot top-K rankings per model and diff them by Kendall tau, with no gold labels. That is a cheap comparison method available to `episode` for the same calibration question, and it is honest about what it does not measure — it detects ranking drift between models, not correctness.

---

### R4 → this change's executive summary

Ideas with no action required, recorded so they are not re-investigated:

- **Idea 2 — resolved negative.** `episode` does not derive scope from `cwd` and does not fragment memory across ADV per-change worktrees. Namespaces are operator-declared `namespace=path` entries (`src/config.rs:147-191`), and `recall`/`remember` take `namespace` as a caller-supplied parameter defaulting to `global` (`src/server.rs`). No defect change opened. The negative result is recorded because the hypothesis was plausible enough to re-raise.
- **Idea 3 — tiered config with unsafe-field stripping.** Applies only if `episode` grows project-local configuration. It has none today: all configuration is environment-driven (`src/config.rs`), and environment variables are already a trusted-operator surface. No action.
- **Idea 5 — embedding size.** 384d at ~22 MB q8 against BGE-large at 1024d. Not a defect on either side. Coupled to R1: it matters only if a brute-force direction is ever revisited, and R1 argues it should not be.
- **Idea 6 — cross-harness `harness` column.** `episode` is ADV/OpenCode-specific by design. Recorded as a known generalization path, not a recommendation.
- **Anti-recommendation, retained.** magic-context's prompt-prefix rewriting via `experimental.chat.messages.transform` is not a technique to copy. Toolbox ADR 0013 measured roughly 15 quota tokens burned per context token saved, because rewriting cached prefix content forces re-billing as `cache_creation`. This does not apply to `episode`, which is an MCP server and never touches the prompt array. It is recorded so nobody later mistakes it for a validated technique.
- **Two designs worth naming, no target.** Persist an embedding only if content hash still matches after the provider call (`saveEmbeddingIfHashMatches`), and never run embedding inference inside a write transaction. Both are cheap correctness properties if `episode` ever moves embedding off the synchronous path.

---

### What this change does not do

- It does not decide the storage direction. R1 supplies evidence to a decision the sibling owns.
- It does not adopt hybrid retrieval. R3 records a candidate bound by the sibling's calibration bar.
- It does not pull auto-promotion into scope. R2 supplies evidence to an item the sibling deferred.
- It modifies no `episode` source file.

### Residual UNVERIFIED

Carried forward without blocking, since none of it changes a routing decision: the full line-level diff between `a32a839` and `a54f9c0` outside the three checked sections; `render-mural` task registration; any committed benchmark output from `ctx-search-benchmark.ts`; and whether the `bge-small-en-v1.5` provider in `MEMORY-DESIGN.md` is implemented or aspirational.
