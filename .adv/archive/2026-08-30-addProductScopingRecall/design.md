# Design — Product-OR-unscoped recall filter

## Root gap

`build_recall_query` compiles `RecallFilters.product` into positive containment `metadata @> {"product": X}` (`src/store.rs:46-48,55-60`). Positive containment requires the key; every untagged row — global wisdom, ingested predecessor knowledge — is excluded. AC1 requires the opposite: a Product scope returns the Product's memories **and** the shared pool.

No test caught this because every fixture row in `tests/recall_filters_it.rs` carries a product tag. The untagged path has zero coverage.

## Decision D1 — the predicate

Replace the product arm of the containment build with:

```sql
(NOT (metadata ? 'product') OR metadata->>'product' = $X)
```

The `?` key-existence idiom is the established convention in this codebase (`PROMOTE_SQL` episodic branch requires key absence; `MARK_CANDIDATES_SQL` guard uses `NOT (metadata ? $3)`). Implementation shape: `product` leaves the shared containment map and becomes its own pushed predicate arm, bound with `push_bind`, never string-built. `work_id`, `tags`, `kinds` keep their existing require-key semantics — only `product` carries shared-pool semantics, because only Product scope is defined as "mine plus shared."

**Semantics, case by case (validator-verified against Postgres 17 docs and jsonfuncs.c):**
- Untagged row, namespace passed → visible.
- Untagged row in `global` → visible. This is the "+ global".
- Row tagged `product = X` → visible.
- Row tagged `product = Y ≠ X`, including in `global` → excluded.
- Row with explicit JSON `null` product → excluded. `?` treats JSON null as present; `->>` returns SQL NULL for a JSON-null value (`jsonb_object_field_text` checks `v->type != jbvNull`, jsonfuncs.c ~L902); `FALSE OR (NULL = X)` → NULL → row excluded. No NULL-propagation trap: untagged evaluates `TRUE OR …` → TRUE.

**Index note (acceptance-review corrected):** the product predicate is not servable by `memories_metadata_gin` — `jsonb_path_ops` does not support bare key-existence, and the `metadata->>'product' = $X` arm is an extracted-text equality over a non-indexed expression. The negative open-followup and promoted exclusions filter without index assistance for the same class of reason; the three share the accepted scan shape. (The design validator's earlier phrasing "serves only containment operators" was broader than the truth — it also serves path operators `@?`/`@@` — and is corrected here and in spec 0010.)

**Combinatorics (validator-traced, review-verified by test):** one production caller (server.rs:130 → store.rs:309); the acceptance review added composed coverage proving the split-out product arm ANDs correctly with the work_id containment and kinds arms.

## Decision D2 — no ingest-side product tagging; product is not a reserved key

Episode never derives Product membership. Tags arrive caller-side via `MemoryContext.product` on `remember` (validated non-blank, `src/types.rs:266-268`). Ingested ADV wisdom stays untagged and lands in the shared pool — correct: predecessor wisdom is per-project with no Product claim, and the namespace arm already separates projects.

A source-supplied `product` key in wisdom metadata is **acceptable data, not a forgery surface** (validator ruling): unlike `promotion_state` — episode-owned state whose forgery hides rows and poisons transitions — a product tag cannot cross the namespace arm and grants nothing that untagged absence did not already grant. Spec 0010 records this stance explicitly so the contrast with `promotion_state` is deliberate, not accidental.

## Decision D3 — spec, doc comment, probe

1. Spec `0010` gains: product-filter semantics section (Product-OR-unscoped, case table, JSON-null stance, index note) and the reserved-key stance above. Both copies committed byte-identical.
2. `RecallFilters.product` (types.rs:240) gains a doc comment stating the Product-OR-unscoped semantics, since JsonSchema exposes no behavior documentation.
3. Probe comment on concord#46 (GitHub), **pre-registered before any outcome can land**: baseline (service restored 2026-08-30 from main 452ffb1, store 0 rows, filter Product-OR-unscoped), method (Product-scoped agent sessions; capture recall calls, namespaces+filters, whether results fit the working context), minimum corpus threshold (40+ memories, 2+ projects of one Product, 10+ tagged) before judging, rubric (sufficient = namespace+product expressed the working context without manual namespace unions or wrong-pool leakage in the transcripts; insufficient = documented failure cases), and judge-before-outcome ordering. Per the user-approved completion bar, AC4 closes on protocol+baseline; the outcome lands later as its own follow-up against the same issue. Posted: https://github.com/Sharper-Flow/concord/issues/46#issuecomment-5471105393

## Verification strategy

RED: new tests seed tagged and untagged rows, query `product: "p"`, assert untagged rows return — fails today (containment drops them). GREEN: predicate swap. Coverage: untagged-visible, tagged-match, wrong-product-excluded, global-namespace composition (untagged global visible, wrong-product global excluded), JSON-null-excluded, and (review-added) composed product+work_id+kinds. Full gate: lib tests, all DB-backed suites, clippy `-D warnings`, fmt, spec copies `cmp`-identical.

## Independent validation

adv-researcher verdict: **APPROVE WITH FINDINGS** — pass, high confidence, low risk, no blockers. Predicate verified against Postgres 17 docs and C source (JSON-null handling exactly as designed); ingest `product` key ruled acceptable data; single production filter caller confirmed. Advisories folded: spec mirror both copies committed, product doc comment, pre-registered probe rubric, reserved-key stance in spec.

Acceptance review (adv-reviewer, contract-correctness): **NEEDS_WORK → resolved**. Two findings fixed by the reviewer in-scope (composed-filter coverage gap; over-broad `jsonb_path_ops` wording in spec and code comment) and one assigned to the orchestrator (the same wording in this design document — corrected in D1 above). Predicate semantics, JSON-null rule, AC2 boundary, and the AC4 completion bar were confirmed unchanged.