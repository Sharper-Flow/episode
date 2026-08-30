# Add promotion state to memories

## Problem

A nascent supersession model already exists in the source data: wisdom entries carry `promoted_at` and `invalidated_by`, and ingest already skips `invalidated_by` entries (`src/ingest.rs:67-70`). But episode doesn't model promotion as first-class. There is no way to:

- mark a memory as a promotion candidate ("this keeps recurring, graduate it to durable knowledge"),
- record that a memory has been promoted and link to where it landed,
- filter OUT promoted memories at recall (so the spec — which now owns that knowledge — doesn't duplicate).

This is the real seam between episodic memory (episode) and durable Product knowledge (Concord specs/decisions). Without it, the two drift and duplicate: a lesson rots in episode while also living in the spec, or never graduates at all.

## The blocker underneath

Discovery found that both ADV fields this change depends on are **post-ingest mutations**, and episode cannot observe either.

`reconcile_root` is write-once per `source_id`: `filter_eligible` (`src/lib.rs:105-113`) drops every item already stored, so `upsert_batch` only ever sees new rows.

- A wisdom entry ingested while valid and later marked `invalidated_by` **stays stored and keeps surfacing**. The parse-time skip prevents adding it, but the only `DELETE` in the codebase (`src/store.rs:274-283`) is restricted to `source = 'manual'`, so ingested rows have no removal path at all.
- A `promoted_at` set after first ingest **never maps**, because dedup skips the row. Since wisdom is overwhelmingly ingested before it is promoted, a parse-time-only mapping would be inert in the common case.

One missing mechanism, two symptoms. Reconcile must apply post-ingest state changes to rows that already exist. Fixing that is a precondition for this feature working at all, so it is in scope rather than deferred.

## Proposed change

### Promotion state type

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
pub enum PromotionState {
    /// Flagged: graduated in ADV, or "this keeps recurring". Still authoritative
    /// in episode, so it stays visible in recall.
    PromotionCandidate {},
    /// Graduated, with the Concord record that now owns the content.
    Promoted { target: String },
}
```

`Episodic` is the default and is represented by the **absence** of the key, not a stored literal. An explicit marker would force a backfill across every row and buy nothing.

`Superseded { by }` was in the original sketch and is dropped. With invalidated rows removed rather than marked, and `forget_manual` covering manual deletion, no caller can produce it.

### Storage

JSONB under the reserved key `promotion_state`, alongside the existing `action` key, filterable via the GIN index from Tier 1b. No migration.

### Recall semantics

- `recall` gains a filter defaulting to exclusion of `Promoted`. An explicit opt-in includes them. This mirrors `include_open_followups` exactly.
- `PromotionCandidate` stays visible by default: it has not graduated, so episode still holds the authoritative copy.

### Transitions

A `promote` tool sets state on an existing row, with the from-state precondition asserted in SQL. Transitions are bidirectional — demotion is the recovery path when a Concord target is later deleted, which would otherwise exclude a memory from recall permanently.

Transitions are **not source-restricted**. The row most needing promotion is an ingested wisdom entry.

### Graduation flow (the seam)

```
ADV sets promoted_at   -->  reconcile marks PromotionCandidate  (still visible)
human supplies target  -->  promote sets Promoted { target }    (excluded from recall)
Concord target dies    -->  demote back to PromotionCandidate   (visible again)
```

## Language and storage assumption

This change is implemented in **Rust + Postgres + pgvector**, per the decision recorded in change `decideStorageLanguageDirection`.

The language choice is safe here, and the receiving contract is what makes it safe. Concord's promotion-receiving contract (`docs/vertical-integration.md:130-158`) crosses a manifest-path + sha256 boundary, holds a `target` explicitly opaque to episode, and records no runtime event on Concord's side. It is language- and storage-agnostic by construction.

## Scope

- `src/types.rs`: `PromotionState` enum, validation, `RecallFilters` field.
- `src/store.rs`: recall predicate, `promote`, `forget_ingested`, `mark_candidates`.
- `src/server.rs`: `promote` tool, recall filter param.
- `src/ingest.rs`: collect invalidated and promoted source ids; map inline promotions.
- `src/lib.rs`: post-ingest state reconciliation step in `reconcile_root`.
- `docs/specs/0011-promotion-state.md` and `.adv/specs/0011-promotion-state.md`, verbatim identical.

## Out of scope

- Auto-promotion heuristics. Manual flagging first; heuristic promotion stays a recorded later candidate.
- Anything on Concord's side. The receiving contract is delivered and unchanged.
- Restoring the episode service.

## The receiving contract

concord#46 Item 2 is **satisfied**. Concord #587 (commit `94989a5`, 2026-08-30) added the contract at `docs/vertical-integration.md:130-158`.

- **Target identity.** A formalized knowledge record of kind `spec` or `decision`, resolved through the knowledge manifest by manifest path + sha256. Unresolvable is `knowledge_missing`. A promotion names a record version, not a moving document.
- **Back-link.** The receiving record cites the originating episode memory in its `sources`. Recorded provenance, not runtime state.
- **Acknowledgement.** Structural only. Concord records no runtime event.
- **Recall exclusion.** Episode's behavior, which AC3 specifies.

**`target` stays opaque.** The contract publishes no serialized format and exposes no resolution surface, so episode cannot verify a target even in principle. Validation is non-blank and nothing more. The accepted failure mode — a malformed target is invisible in episode and rejected at Concord — is mitigated by the two-step flow: nothing is hidden from recall until a human supplies a real target.

## Dependencies

Both declared prerequisites have **shipped**, though neither retains an ADV change record:

- `enrichRememberWithContext` — `RememberParams.context: Option<MemoryContext>` in `src/server.rs`.
- `addRecallMetadataFilters` — `RecallParams.filters: Option<RecallFilters>` in `src/server.rs`, with `RecallFilters` at `src/types.rs:131-140`, spec `0010`, and `tests/recall_filters_it.rs` on disk.

This change is **not blocked**.

## Known operating condition

Episode is not running. The deployed binary predates migration `0002_metadata_gin.sql`, so `sqlx` refuses to start, and the store holds zero rows. Both defects above are therefore **latent, not active** — they become live on the first reconcile after the service is restored, which is why this should land before it comes back up. DB-backed tests run against the same healthy store and are unaffected.

## Acceptance criteria

- AC1: `PromotionState` exists with two stored variants, `PromotionCandidate` and `Promoted { target }`. `Episodic` is the absent-key default and is never written.
- AC2: New memories carry no `promotion_state` key and therefore default to episodic.
- AC3: `recall` excludes `Promoted` rows by default.
- AC4: An explicit opt-in filter includes them. `PromotionCandidate` is visible in both modes.
- AC5: A memory can be transitioned between states through the `promote` tool, in both directions, on ingested as well as manual rows. A from-state mismatch affects zero rows.
- AC6: Reconcile applies post-ingest ADV state to already-stored rows: `invalidated_by` removes the row, and `promoted_at` sets `PromotionCandidate`.
- AC7: Reconcile is idempotent. A human-set `Promoted { target }` survives subsequent reconciles and is never clobbered back to candidate.
- AC8: `target` is stored opaquely and validated only as non-blank.
- AC9: Tests cover each transition, the recall default-exclusion, both defect regressions, and idempotency.

## Epic context

Member of Epic `shapeEpisodeStructuredMemory` (advisory order 5 of 6). Implements the episode-to-durable-knowledge promotion seam.

Both Concord-side questions this change touches are settled. The receiving contract (concord#46 Item 2) is satisfied as of 2026-08-30. The ownership question, C20, is **resolved**: Concord's operator approved episode staying external, optional, and Product-scoped when configured on 2026-08-30, with the product-scoping probe demoted to a reopen trigger. Concord PR #596 executes the documentation deliverables. Neither question gates this change.

## External evidence — magic-context

> Routed from change `studyMagicContextMemoryDesign`. Source: [`cortexkit/magic-context`](https://github.com/cortexkit/magic-context) (MIT), read at HEAD `a54f9c0`. This section attaches to the **already-deferred** "auto-promotion heuristics" item under *Out of scope*. It does not pull that item into scope and it does not modify the acceptance criteria. Manual flagging remains this change's design.

magic-context has a working implementation of the heuristic promotion this change defers. Recording its mechanics now means the deferred item, when it activates, starts from a validated design rather than a blank page.

### Repeated-confirmation promotion

The rule is two-axis, and the two-axis shape is the transferable part. From `primer-clustering.ts`:

```ts
export const PRIMER_CLUSTER_THRESHOLD = 0.85;
export const PRIMER_CLUSTER_HYSTERESIS = 0.02;
export const PRIMER_PROMOTION_THRESHOLD = 2;
export const PRIMER_MIN_SPAN_DAYS = 7;

// ...
return summary.support >= threshold && summary.spanDays >= minSpanDays;
```

The two axes are **not** what the constant names suggest, so read `summarizePrimerCluster` before reusing them:

- Candidates are first deduplicated by occurrence key, then collapsed to **one candidate per distinct UTC day**. `support` is the count of those distinct days — not the count of occurrence keys.
- `spanDays` is the **elapsed** whole-day distance between the earliest and latest surviving candidate, not a count of days on which something happened.

So the rule reads: promote when a cluster has been confirmed on **at least 2 distinct days** *and* those confirmations are **at least 7 elapsed days apart**. Clustering is by cosine ≥ 0.85, with the threshold relaxed by 0.02 for candidates already inside an existing primer, which is what stops membership flapping at the boundary.

Requiring both axes is what makes it resist the two obvious false positives. A single chatty session collapses to one distinct day and fails `support`. A burst of activity over one busy afternoon fails `spanDays`. Neither axis alone rejects both.

A simpler second path also exists: user-memory observations promote after ≥ 3 observations (`task-executor.ts` ~L434, `promotionThreshold ?? 3`).

### Deduplication is exact, not fuzzy

Before promotion, duplicates are collapsed by **normalized content hash per category**, backed by `idx_memories_project_category_hash`. Cross-category merges are structurally rejected rather than scored.

This is worth noting against the intuition that a semantic store should dedupe semantically. They use cosine to find promotion *candidates* and an exact hash to collapse *duplicates*. The two jobs get different mechanisms.

### Keep-versus-discard is verified, not inferred

Their `verify` task gates each memory on whether its **backing file changed** since `verified_at`. Partial progress persists across runs, and refusals count as non-failing skips rather than errors.

This transfers directly. episode ingests from ADV wisdom and reflection files, so every ingested memory already has a backing file. The same file-change gate is available without new infrastructure, and it is a stronger signal than age-based decay: a gotcha about a module that has not changed is not stale merely because it is old.

### Concurrency, if a background pass is ever added

Dreamer tasks are assigned **lease domains** in `dreamer/task-registry.ts`, so disjoint-state tasks run concurrently while memory-mutating tasks serialize on a shared `memory:<project>` lease. Read-then-write runs atomically under `BEGIN IMMEDIATE`. The Postgres analogue is an advisory lock.

The design point is not the specific primitive. It is that a consolidation pass mutates the same rows that `recall` and `remember` are serving live, and needs an explicit exclusion mechanism rather than optimistic hope.

### Model-pinning caveat — prospective, and narrower than first reported

This change has no LLM-backed pass today, so the following constrains the deferred item rather than the current design.

magic-context's **historian** appends the live session provider/model as an absolute last resort after its configured fallback chain (`compartment-runner-historian.ts`, in `runFallbackHistorianPass`). `ARCHITECTURE.md` labels this behavior "historian-only".

It is **refuted** for the other two background roles: `dreamer` and `sidekick` both resolve through configured `fallback_models` only, and throw when that list is exhausted (`resolve-fallbacks.ts`, `shared/model-resolution.ts`, `sidekick/agent.ts`).

**Reader trap worth carrying with the citation:** `resolve-fallbacks.ts`'s header comment loosely describes "the runner's session-model last resort" as though it applied to all hidden agents. Only the historian runner implements it. Anyone consulting that comment instead of the runners will conclude the fallback is general. It is not.

The constraint that lands: **if episode ever adds an LLM-backed consolidation pass, pin its model explicitly and fail closed.** Never fall back to the caller's session model. On a metered subscription that silently bills the user's scarce quota to do background maintenance they did not ask for at that moment, and the failure is invisible — the pass appears to succeed.
