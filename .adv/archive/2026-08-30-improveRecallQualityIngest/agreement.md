# Agreement — improve recall quality and ingest source abstraction

## Objectives

1. Recall can narrow by provenance: `sources` filters on the typed source column with the same ergonomics as `kinds` (one field, not a metadata object).
2. Recall can exclude stale memories: `max_age_days` drops rows first captured longer ago than the cutoff. Pure filter — ranking and scores untouched.
3. Ingest is source-agnostic: a parser-shaped trait hosts ingest sources; ADV wisdom and ADV reflections become two implementations of it; reconcile aggregates over a source list.
4. A future Concord lesson parser (CD-0026: git-backed markdown + manifest, a different shape than JSONL) can be added as a new implementation without touching reconcile.

## Acceptance criteria

- AC1: `recall` accepts `sources` as a top-level filter param over the closed set `manual` / `adv_wisdom` / `adv_reflection`; unknown values reject; composes with AND alongside every existing filter and the namespace arm. *(Narrowed from the original proposal: `kinds` already ships in `RecallFilters`.)*
- AC2: `recall` accepts `max_age_days`; rows whose `created_at` is older than the cutoff are excluded; `0` rejects as invalid, not as no-op.
- AC3: No score-blending weights are introduced. Recency is a filter, not a ranking input. The hybrid-retrieval candidate (proposal §External evidence) stays a recorded candidate.
- AC4: Ingest is source-agnostic via a parser trait; ADV is impls, not the contract.
- AC5: Current ADV ingestion behavior is preserved bit-for-bit at the store boundary: retraction before dedup, promotion marking after the batch loop, per-source error isolation (one source failing warns and does not block the others), write-once dedup, idempotent repeated reconciles.
- AC6: Tests cover each sub-part: sources filter (validation + DB), max_age (DB with seeded timestamps), trait aggregation (unit), and every existing ingest/reconcile/promotion suite passes unchanged.

## Constraints

- `created_at` is the recency basis: first-capture time. Ingest is write-once so it never moves; promotions touch only `updated_at`. A re-ingested corrected lesson keeps its original capture time by design.
- No schema change: `source` and `created_at` columns exist.
- No new index (same accepted scan-shape reasoning as spec 0010's unindexed predicates).
- Spec 0010 is the filter contract — both copies stay byte-identical. The ingest trait gets a small spec 0012 (durable contract for external parsers).

## Avoidances

- Do not re-implement the `kinds` filter or treat 3a as unfinished — half of it shipped with spec 0010.
- Do not build the Concord lesson parser. The trait is the deliverable; the parser is separate future work.
- Do not adopt hybrid retrieval (3d) — AC3's calibration bar governs it.