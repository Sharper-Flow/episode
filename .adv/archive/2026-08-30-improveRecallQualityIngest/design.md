# Design — source filter, recency filter, ingest source trait

## Sub-part 3a — `sources` filter

`RecallFilters.sources: Option<Vec<MemorySource>>`. `MemorySource` already carries `#[serde(rename_all = "snake_case")]` and `as_str()` (src/types.rs:32-43); it gains `JsonSchema` so the closed set renders in the MCP schema. Unknown wire values reject via serde — rmcp maps deserialization failure to `invalid_params` (validator-verified against rmcp 2.2.0 source). `sources: Some(vec![])` rejects as invalid (empty list), matching spec 0010's reject-don't-default pattern.

SQL: `source = ANY($n)` binding `Vec<String>` via `as_str()` — the exact shape of the `kinds` arm (store.rs `build_recall_query`). AND-composed. Typed-column predicate, unindexed like `kinds = ANY` — same accepted scan shape.

## Sub-part 3b — `max_age_days` filter

`RecallFilters.max_age_days: Option<u32>`. Validation: `Some(0)` rejects before embedding ("must be positive"). u32 at the type boundary rejects negative wire values at deserialization.

SQL: `created_at >= now() - make_interval(days => $1)` binding **i32 via `i32::try_from`** (validator blocking correction): sqlx-postgres 0.9 implements Encode for i8/i16/i32/i64 only — u32 does not compile — and an i64 bind fails `make_interval`'s int4 parameter because function arguments accept implicit coercions only (PG typeconv-func). The `interval '1 day'`-multiply and `|| ' days'` alternatives add a coercion hop or a text round-trip; rejected.

Basis is `created_at` — first-capture time. Insert never includes it (store.rs:225-227); the conflict path sets only `updated_at`; promotions touch metadata + `updated_at`. One documented nuance for spec 0010: retract-then-reinstate recreates the row and resets `created_at` — correct, since reinstatement is genuinely a fresh capture of re-validated knowledge. Pure WHERE filter; scores and ranking untouched (AC3 satisfied by omission).

## Sub-part 3c — ingest source trait

One shape replaces two divergent return types:

```rust
pub struct SourceParse {
    pub items: Vec<MemoryInput>,
    pub invalidated: Vec<String>,  // retraction: source ids withdrawn upstream
    pub promoted: Vec<String>,     // graduation recorded upstream
}

pub trait IngestSource {
    /// Parse one project root for one namespace. Implementations resolve
    /// their own paths and file formats; reconcile owns store effects.
    fn parse(&self, namespace: &str, project_root: &Path) -> Result<SourceParse>;
}
```

- `AdvWisdomSource` wraps the existing `parse_wisdom` body; `WisdomParse` folds into `SourceParse` (validator-verified safe: only ingest.rs names the type; all consumers use the fields, which are identical).
- `AdvReflectionSource` wraps `parse_reflections`; reflections report empty `invalidated`/`promoted` — already their effective contract, and the same items-only shape a CD-0026 Concord lessons impl will have (validator traced CD-0026: working-tree markdown + manifest read by path; git authority is publish-side; idempotent publish with no retraction stream).
- `reconcile_root` replaces the two `match` blocks with a loop over `const SOURCES: &[&dyn IngestSource] = &[&AdvWisdomSource, &AdvReflectionSource]`, aggregating in that order with warn-and-continue per source — bit-for-bit today's semantics. Validator confirmed item order is not load-bearing (dedup is HashSet-based; batch boundaries only), and it is preserved regardless.
- Sync trait with `&dyn`: async fn in traits is not dyn-compatible, and no current or evidenced consumer needs it; a future blocking-IO parser dispatches via `spawn_blocking` inside its impl without a trait change.
- The source list is a hardcoded slice; a registration surface arrives with the first external parser that needs one.

**Spec 0012 records two store-level facts the trait contract needs** (validator findings): retraction (`forget_ingested`) and promotion marking (`MARK_CANDIDATES_SQL`) hard-restrict to `source IN ('adv_wisdom','adv_reflection')` — future sources are items-only by construction until that restriction is deliberately widened; and `existing_source_ids` dedups on (namespace, source_id) regardless of source — every source must own a disjoint id space or rows collide at the dedup boundary.

## Sub-part 3d — explicitly not adopted

Hybrid lexical retrieval stays a recorded candidate under AC3's calibration bar. Nothing here touches tsvector, pg_trgm, or ranking.

## Specs

1. Spec 0010 (both tracked copies, byte-identical): Filter Contract gains `sources` (closed set, typed-column ANY, empty list rejects) and `max_age_days` (created_at basis, positive-only, filter-not-rank, reset-on-reinstate note); Verification extends.
2. New spec `0012-ingest-source-contract.md` (both copies): the trait, SourceParse, ownership rules (sources report, reconcile applies), per-source error isolation, items-only semantics for non-ADV sources with the store-restriction citation, disjoint id-space requirement, write-once semantics unchanged, and the CD-0026 future-consumer rationale for the parser shape.

## What deliberately does not change

- Store methods: `forget_ingested`, `mark_candidates`, `existing_source_ids`, `upsert_batch` — untouched.
- Spec 0011's ingest contract: write-once per source_id, reserved-key stripping, retraction/promotion semantics — preserved (AC5 is the existing suite corpus passing unchanged).
- MCP tool schemas other than RecallFilters' two new optional fields.

## Verification strategy

3a: unit tests (unknown source string rejects; empty list rejects; all three values accepted) + DB test seeding manual/adv_wisdom/adv_reflection rows and filtering each. 3b: DB test backdating `created_at` via SQL and asserting cutoff boundaries; `0` rejects in unit validation. 3c: unit test asserting the SOURCES loop aggregates both impls and isolates a failing source (a stub source returning Err); existing reconcile/promotion/ingest DB suites are the AC5 proof. Full gate: lib, all DB-backed, clippy `-D warnings`, fmt, spec copies cmp-identical (0010 and 0012).

## Independent validation

adv-researcher verdict: **APPROVE WITH FINDINGS** (caution, high confidence, low risk). One blocking correction adopted: the u32 SQL bind is impossible under sqlx-postgres 0.9 (no Encode impl) and an i64 bind fails make_interval's int4 parameter under function-arg coercion rules — the design now binds i32 via i32::try_from. Advisories adopted: reject empty sources list; document created_at reset-on-reinstate in spec 0010; record ADV-scoped retraction/promotion and the disjoint id-space requirement in spec 0012; bind Vec<String> via as_str mirroring kinds; preserve aggregation order. The trait cut, WisdomParse fold, created_at basis, and wire rejection were all verified sound against traced code, the CD-0026 decision text, and sqlx/rmcp sources.