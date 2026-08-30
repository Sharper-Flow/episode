# Archive Briefing Digest

**Change ID:** addPromotionStateMemories
**Title:** Add promotion state to memories
**Status:** archived
**Generated:** 2026-08-30T18:49:15.324Z

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

Epic: shapeEpisodeStructuredMemory · Add promotion state to memories (order 5)

## Durable Facts

Showing 29 of 29 durable facts.

- **[report_follow_up]** follow_ups: Record the no-reverse-transition decision (demote tool vs accepted permanent exclusion) explicitly in the spec.
- **[report_follow_up]** follow_ups: Correct the design's present-tense retracted-knowledge claim to future tense (zero rows, service down).
- **[report_follow_up]** follow_ups: Spec 0011 should enumerate the valid transition table, including whether Episodic→Promoted direct is allowed.
- **[research_citation]** sources: src/ingest.rs: parse_wisdom skips invalidated_by entries at parse time only; no deletion of already-stored rows; metadata = raw object minus content (lines 87-90). (src/ingest.rs:67-70)
- **[research_citation]** sources: src/lib.rs: reconcile_root is upsert-only (existing_source_ids + upsert_batch); filter_eligible drops items whose source_id already exists, so upsert_batch never touches existing rows. (src/lib.rs:159-255,105-113)
- **[research_citation]** sources: src/store.rs: Sole DELETE is forget_manual restricted to source='manual'. Negative open-followup exclusion predicate precedent at 66-72. upsert_batch full-overwrite ON CONFLICT at 180-190. (src/store.rs:274-283,66-72,146-195)
- **[research_citation]** sources.omitted: 5 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: Defect CONFIRMED end to end: skip at ingest.rs:67-70 is parse-time only; reconcile_root (lib.rs:159-255) has no prune; sole DELETE is forget_manual (store.rs:274-283) restricted to source='manual'; no cascade/TTL/reset anywhere (grep over src; migrations 0001/0002). Blast radius is LATENT: store holds zero rows and service is down (problem statement), so the design's present-tense claim 'Episode currently serves knowledge that ADV has explicitly retracted' overstates — it becomes true on first reconcile after service restoration. The proposed delete-on-invalidated fix is correct and self-healing (un-invalidation re-ingests fresh). Idempotency: the feared overwrite does not exist — filter_eligible (lib.rs:105-113) makes upsert_batch unreachable for existing rows, so tool-set promotion_state survives re-ingest. The REAL hole is inverse: promoted_at set in ADV after first ingest never maps (dedup skips the row), making the ingest-side AC6 mechanism inert for the common case; the planned parse_wisdom unit test cannot catch this. Concurrency: single-statement UPDATE with SQL from-state predicate is sufficient (row lock + READ COMMITTED predicate recheck); metadata write must stay in SQL. JSONB storage matches the ActionState precedent; RecallFilters additive field is backward-compatible (#[serde(default)], not persisted); PromotionCandidate {} empty variant has exact house precedent (LinkedWork/OpenFollowup, proven by server.rs:447-506 schema test). Design-internal contradiction: 'promote mirrors forget_manual' (manual-only) vs deferred question assuming humans promote ingested rows. AC set (AC1 four variants, AC3 excludes Superseded) conflicts with the three-variant design. No reverse transition exists for a Promoted row whose target dies — permanent silent exclusion, a failure mode the design's own D1 analysis predicts.
- **[unresolved_action]** required_main_agent_actions: Preserve and checkpoint the five reviewed file edits before acceptance.
- **[unresolved_action]** required_main_agent_actions: Do not revisit reflection reconciliation, reconcile ordering, promoted_at stripping, or log-and-retry behavior. The inspected contracts and tests support their current behavior.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] JSON path extraction returns SQL NULL for an absent key, JSON null, and malformed objects. A NULL-safe tag comparison cannot enforce an absent-key state invariant. Pair the NULL parameter with an explicit key-existence predicate.
- **[archive_only_evidence]** changes_made: src/store.rs: Fixed the promotion compare-and-set predicate so Episodic matches only an absent promotion_state key. JSON null and malformed objects no longer pass the precondition.
- **[archive_only_evidence]** changes_made: tests/promotion_state_it.rs: Added a Postgres regression test for present JSON null and empty-object promotion states with an Episodic from-state.
- **[archive_only_evidence]** changes_made: src/types.rs: Updated PromotionStateKind documentation to describe the exact absent-key SQL branch.
- **[archive_only_evidence]** changes_made: docs/specs/0011-promotion-state.md: Corrected the transition contract to distinguish stored-state tag comparison from exact episodic absent-key matching.
- **[archive_only_evidence]** changes_made: .adv/specs/0011-promotion-state.md: Kept the ADV specification copy byte-identical with the durable documentation copy.
- **[archive_only_evidence]** verification: tests_run=cargo test --test promotion_state_it promote_rejects_present_state_when_from_is_episodic -- --ignored (RED before fix), cargo test --test promotion_state_it promote_rejects_present_state_when_from_is_episodic -- --ignored (GREEN after fix), cargo test --lib, cargo test --tests -- --ignored, cargo clippy --all-targets -- -D warnings, cargo fmt --check, cmp -s docs/specs/0011-promotion-state.md .adv/specs/0011-promotion-state.md, git diff --check results=pass — The regression produced updated=1 before the fix and updated=0 after it. Final verification passed: 63 unit tests, all ignored Postgres suites including 5 promotion and 5 reconcile tests, clippy with warnings denied, format, byte identity, and diff whitespace.
- **[unresolved_action]** required_main_agent_actions: Keep restoration single-instance. Before supporting overlapping instances, add per-namespace cross-process reconcile serialization and a stale-retraction regression.
- **[unresolved_action]** required_main_agent_actions: Choose measured limits before bounding source-id mutation chunks or promotion target bytes.
- **[unresolved_action]** required_main_agent_actions: Monitor EXPLAIN plans and recall cardinality as promoted-row fraction grows; add a plan regression only when a representative corpus and threshold exist.
- **[unresolved_action]** required_main_agent_actions: Do not revisit the settled episodic precondition or source promotion_state stripping defects.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] An ingestion dedup check does not prevent ON CONFLICT races. Preserve service-owned metadata during the conflict update because a state transition can complete between lookup and write.
- **[archive_only_evidence]** changes_made: src/store.rs: Preserved an existing promotion_state in the ingestion conflict path and clarified kind-level compare-and-set semantics.
- **[archive_only_evidence]** changes_made: tests/promotion_state_it.rs: Added a regression that promotes a row between competing ingestion writes and proves the second upsert preserves the target.
- **[archive_only_evidence]** changes_made: src/server.rs: Corrected MCP parameter and tool text to state that same-kind transitions are last-writer-wins.
- **[archive_only_evidence]** changes_made: docs/specs/0011-promotion-state.md: Recorded ingestion conflict preservation and exact kind-level concurrency semantics.
- **[archive_only_evidence]** changes_made: .adv/specs/0011-promotion-state.md: Kept the mirrored promotion-state specification identical.
- **[archive_only_evidence]** verification: tests_run=EPISODE_TEST_DATABASE_URL='postgres://episode:episode@localhost:5434/episode' cargo test --test promotion_state_it concurrent_ingest_conflict_preserves_promotion_state -- --ignored --exact (RED, then GREEN), cargo test --lib, EPISODE_TEST_DATABASE_URL='postgres://episode:episode@localhost:5434/episode' cargo test --tests -- --ignored, cargo clippy --all-targets -- -D warnings, cargo fmt --check, diff -q docs/specs/0011-promotion-state.md .adv/specs/0011-promotion-state.md, git diff --check results=pass — Regression failed before the SQL repair with left None and right Promoted, then passed. Final checks passed: 63 library tests, 19 ignored Postgres tests, clippy, formatting, mirrored-spec comparison, and diff validation.
- **[epic_terminal_note]** epic.membership: shapeEpisodeStructuredMemory · Add promotion state to memories (order 5)

## Contract / AC Coverage

No contract items.

## Unresolved Actions

- Preserve and checkpoint the five reviewed file edits before acceptance.
- Do not revisit reflection reconciliation, reconcile ordering, promoted_at stripping, or log-and-retry behavior. The inspected contracts and tests support their current behavior.
- Keep restoration single-instance. Before supporting overlapping instances, add per-namespace cross-process reconcile serialization and a stale-retraction regression.
- Choose measured limits before bounding source-id mutation chunks or promotion target bytes.
- Monitor EXPLAIN plans and recall cardinality as promoted-row fraction grows; add a plan regression only when a representative corpus and threshold exist.
- Do not revisit the settled episodic precondition or source promotion_state stripping defects.
