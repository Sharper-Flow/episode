# Archive Briefing Digest

**Change ID:** improveRecallQualityIngest
**Title:** Improve recall quality and ingest abstraction
**Status:** archived
**Generated:** 2026-08-30T22:21:34.307Z

## Identity Anchors

- CHANGE
- STATUS
- TERMINAL_GATE_SUMMARY

## Archive Digest

**Status:** archived

| Gate | Status |
| --- | --- |
| proposal | done |
| discovery | done |
| design | done |
| planning | done |
| execution | done |
| acceptance | done |
| release | pending |

## Epic Context

Epic: shapeEpisodeStructuredMemory · Improve recall quality and ingest abstraction (order 6)

## Durable Facts

Showing 30 of 30 durable facts.

- **[report_follow_up]** follow_ups: Design edit: i32 bind for make_interval days parameter with i32::try_from conversion.
- **[report_follow_up]** follow_ups: Planning check: locate the byte-identical counterpart of docs/specs/0010 for the cmp gate.
- **[report_follow_up]** follow_ups: Spec 0012: document items-only semantics for non-ADV sources (source-scoped retraction/promotion) and the namespace-scoped source_id dedup expectation.
- **[research_citation]** sources: store.rs (worktree): kinds=ANY arm template; upsert_batch insert list excludes created_at, conflict path sets updated_at only; PROMOTE_SQL/MARK_CANDIDATES_SQL touch metadata+updated_at, both restrict source IN ('adv_wisdom','adv_reflection'); forget_ingested same restriction; existing_source_ids matches namespace+source_id without source scoping. (src/store.rs:24-108,116-136,207-309,369-414)
- **[research_citation]** sources: lib.rs (worktree): reconcile_root aggregation wisdom-then-reflections with per-source warn-and-continue; retraction before dedup; mark-after-batch; partition/filter order-preserving. (src/lib.rs:98-131,159-306)
- **[research_citation]** sources: types.rs (worktree): MemorySource 3 variants, snake_case serde, as_str(); RecallFilters serde(default,deny_unknown_fields)+JsonSchema; tags/kinds reject blank+empty lists. (src/types.rs:30-50,236-300)
- **[research_citation]** sources.omitted: 8 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: 1) TRAIT CUT: right-sized for the evidenced consumer. CD-0026 lessons are working-tree markdown + a JSON manifest read by path; git authority is publish-side only, so a sync parse(namespace, project_root) needs no repo state or network. async fn in traits is not dyn-compatible, so &[&dyn IngestSource] with sync methods is the boring choice that avoids async_trait boxing; a future git-heavy impl runs under spawn_blocking at the call site without a trait change. CD-0026 D1/D2 (idempotent publish, scope-based promotion, no retraction) means a Concord impl reports items-only — same effective contract as reflections today (empty invalidated/promoted), so SourceParse generalizes. 2) WISDOMPARSE FOLD: safe. Type name appears only in ingest.rs (def:51, sig:78, ctor:151) and archived .adv history records (not compiled); lib.rs consumes parse_wisdom by field appends only (lib.rs:169-184); ingest tests access .items/.invalidated/.promoted — field-identical to SourceParse. 3) CREATED_AT: verified stable. Insert list excludes it (store.rs:225-227, default now()); conflict path (242-261), PROMOTE_SQL (116-123), MARK_CANDIDATES_SQL (131-136) touch updated_at/metadata only; forget_ingested is DELETE. Nuance: retract-then-reinstate (documented self-healing, store.rs:364-366) recreates the row with a NEW created_at — it is 'first capture of the current stored copy', not 'first capture ever'; spec 0010 wording should say so. 4) MAKE_INTERVAL: design flaw — sqlx-postgres 0.9 has NO u32 Encode impl (int.rs: i8/i16/i32/i64 only), so push_bind(u32) does not compile; and an i64 bind into make_interval(days=>$n) risks 'function does not exist' because int8->int4 is assignment-only, not implicit (PG typeconv-func + CREATE CAST). Bind i32: exact INT4 signature match; convert u32 wire value via i32::try_from with a validation error. interval-multiplication and text||' days' alternatives both add a coercion or text round-trip — reject. 5) SOURCES FILTER: mechanism verified. MemorySource needs JsonSchema + schemars(crate) derive (pattern at types.rs:57-59); schemars honors serde rename_all (in-tree proof server.rs:517-530); rmcp Parameters is serde-transparent and maps deserialization failure to invalid_params (rmcp tool.rs:181-190) — unknown variants reject at the wire like ActionState shapes today. SQL arm must bind Vec<String> via as_str() (MemorySource has no sqlx impl), mirroring kinds=ANY (store.rs:72-78). 6) AC5: item order affects only batch partition boundaries — dedup is HashSet-based, ids and scores are order-free — so correctness never depends on wisdom-before-reflections; bit-for-bit AC5 does. Loop over SOURCES=[wisdom, reflections] in that order with per-source warn-and-continue reproduces today's semantics exactly. Two scoping facts for spec 0012: forget_ingested and MARK_CANDIDATES_SQL hard-restrict source IN ('adv_wisdom','adv_reflection') (store.rs:376,135) — a future non-ADV source's items would ingest (upsert_batch writes input.source) but its invalidated/promoted reports would be silently ignored; and existing_source_ids matches namespace+source_id WITHOUT source (store.rs:300-301), so cross-source source_id collisions in a namespace false-positive dedup — spec 0012 should state the disjoint-id-space expectation for new sources.
- **[unresolved_action]** required_main_agent_actions: Checkpoint the scoped review fixes before acceptance.
- **[unresolved_action]** required_main_agent_actions: Use this report as the contract-correctness acceptance evidence.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] A u32 request field does not imply that PostgreSQL can bind its full range. Validate against the target SQL argument type at the public store boundary before QueryBuilder conversion.
- **[archive_only_evidence]** changes_made: src/types.rs: Rejected max_age_days values above i32::MAX so every accepted value fits PostgreSQL make_interval's int4 argument.
- **[archive_only_evidence]** changes_made: src/store.rs: Validated filters at the public store boundary and extended bound-SQL coverage through sources and max_age_days.
- **[archive_only_evidence]** changes_made: src/server.rs: Added validation coverage for u32::MAX max_age_days input.
- **[archive_only_evidence]** changes_made: tests/recall_filters_it.rs: Exercised all recall filters in one query and proved the store rejects an unbindable day count without panic.
- **[archive_only_evidence]** changes_made: src/ingest.rs: Removed stale main.rs and Vec-return documentation and removed an invalid spawn_blocking claim from the synchronous trait contract.
- **[archive_only_evidence]** changes_made: src/lib.rs: Changed the reconcile documentation from two ADV sources to configured ingest sources.
- **[archive_only_evidence]** changes_made: docs/specs/0010-recall-metadata-filters.md: Documented the int4 day range and corrected the stale boundary that said this capability added no recency behavior.
- **[archive_only_evidence]** changes_made: .adv/specs/0010-recall-metadata-filters.md: Kept the tracked spec copy identical after the recency contract corrections.
- **[archive_only_evidence]** changes_made: docs/specs/0012-ingest-source-contract.md: Removed the unsupported claim that a synchronous parser can dispatch and await spawn_blocking internally.
- **[archive_only_evidence]** changes_made: .adv/specs/0012-ingest-source-contract.md: Kept the tracked spec copy identical after the trait contract correction.
- **[archive_only_evidence]** verification: tests_run=cargo test --lib, EPISODE_TEST_DATABASE_URL='postgres://episode:episode@localhost:5434/episode' cargo test --tests -- --ignored, cargo clippy --all-targets -- -D warnings, cargo fmt --check, cmp '.adv/specs/0010-recall-metadata-filters.md' 'docs/specs/0010-recall-metadata-filters.md', cmp '.adv/specs/0012-ingest-source-contract.md' 'docs/specs/0012-ingest-source-contract.md', git diff --check results=pass — 64 unit tests and 22 ignored Postgres tests passed. Clippy, rustfmt, both spec-pair comparisons, and diff checks passed. The SQL integration query activated namespace, product, work_id, tags, kinds, sources, max_age_days, open-followup, and promoted predicates together. QueryBuilder preserves bind order by construction. now() is statement/transaction stable, TIMESTAMPTZ arithmetic has no session-timezone cutoff shift, and the 100-day fixture margin avoids boundary timing races. AC5 is supported by unchanged store methods, unchanged parser bodies, aggregation isolation coverage, and passing reconcile/promotion/ingest suites. Changed warning text is a log surface, not store-boundary behavior.
- **[unresolved_action]** required_main_agent_actions: Checkpoint the scoped fixes in src/store.rs and tests/recall_filters_it.rs.
- **[unresolved_action]** required_main_agent_actions: Treat repeated ingest-source warning control or failure metrics as an operational follow-up if stderr volume or alert coverage is insufficient.
- **[unresolved_action]** required_main_agent_actions: Deploy with a normal service restart, then inspect the first reconcile pass. No migration or coordinated database action is required. A binary-only rollback is safe.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] PostgreSQL make_interval(days => 2147483647) succeeds, but subtracting that interval from now() raises 'timestamp out of range'. Compare now() - created_at to the interval when the accepted day range can exceed the timestamptz calendar range.
- **[archive_only_evidence]** changes_made: src/store.rs: Rewrote max-age comparison as interval-to-interval arithmetic. PostgreSQL accepted make_interval(i32::MAX) but the former timestamp subtraction failed with 'timestamp out of range'. The new expression preserves filter semantics without producing an out-of-range timestamp.
- **[archive_only_evidence]** changes_made: tests/recall_filters_it.rs: Added coverage that recalls all seeded rows with max_age_days = i32::MAX, which is the largest contract-valid value.
- **[archive_only_evidence]** verification: tests_run=docker exec episode-db psql -U episode -d episode -v ON_ERROR_STOP=1 -c "SELECT make_interval(days => 2147483647) AS max_interval;" -c "SELECT now() - make_interval(days => 2147483647) AS cutoff;", cargo test --test recall_filters_it sources_and_max_age_filter_recall -- --ignored --exact, cargo test --lib, cargo test --tests -- --ignored, cargo clippy --all-targets -- -D warnings, cargo fmt --check, cmp .adv/specs/0010-recall-metadata-filters.md docs/specs/0010-recall-metadata-filters.md, cmp .adv/specs/0012-ingest-source-contract.md docs/specs/0012-ingest-source-contract.md, cargo rustc --lib -- --emit=llvm-ir=target/episode-review.ll results=pass — The PostgreSQL probe reproduced 'timestamp out of range' for the former expression. The focused test then passed with the interval comparison. Unit tests passed: 64. Ignored database suites passed: 22 tests across six suites. Clippy, rustfmt, both spec comparisons, and git diff checks passed. LLVM IR lines 2341-2343 show two constant vtables and one constant trait-object slice payload.
- **[epic_terminal_note]** epic.membership: shapeEpisodeStructuredMemory · Improve recall quality and ingest abstraction (order 6)

## Contract / AC Coverage

No contract items.

## Unresolved Actions

- Checkpoint the scoped review fixes before acceptance.
- Use this report as the contract-correctness acceptance evidence.
- Checkpoint the scoped fixes in src/store.rs and tests/recall_filters_it.rs.
- Treat repeated ingest-source warning control or failure metrics as an operational follow-up if stderr volume or alert coverage is insufficient.
- Deploy with a normal service restart, then inspect the first reconcile pass. No migration or coordinated database action is required. A binary-only rollback is safe.
