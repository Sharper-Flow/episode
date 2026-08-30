# Archive Briefing Digest

**Change ID:** addProductScopingRecall
**Title:** Add product scoping to recall
**Status:** archived
**Generated:** 2026-08-30T21:30:33.240Z

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

Epic: shapeEpisodeStructuredMemory · Add product scoping to recall (order 4)

## Durable Facts

Showing 28 of 28 durable facts.

- **[report_follow_up]** follow_ups: Add doc comment on RecallFilters.product describing Product-OR-unscoped semantics (surfaces via schemars to MCP clients).
- **[report_follow_up]** follow_ups: Execution note: update and cmp the untracked .adv/specs/0010 mirror in the main checkout; worktrees will not see it.
- **[report_follow_up]** follow_ups: Probe outcome follow-up (concord#46): pre-register judging rubric, minimum evidence corpus/window, and named judge before recording sufficient/insufficient.
- **[report_follow_up]** follow_ups: Document the ingest reserved-key stance in spec 0010: episode owns promotion_state (+ structural content/promoted_at); caller-domain keys persist verbatim.
- **[research_citation]** sources: PostgreSQL 17 docs, 9.1 Logical Operators (three-valued truth tables): TRUE OR NULL = TRUE; FALSE OR NULL = NULL; NOT NULL = NULL. Grounds both the untagged-visible and JSON-null-excluded rows. (https://www.postgresql.org/docs/17/functions-logical.html)
- **[research_citation]** sources: PostgreSQL docs, jsonb operators table (Table 9.46, ? operator): jsonb ? text: does the text string exist as a top-level key or array element. Key existence, value not inspected — JSON null product still answers TRUE. (https://www.postgresql.org/docs/17/functions-json.html)
- **[research_citation]** sources: PostgreSQL source: jsonb_object_field_text (implementation of jsonb ->> text): Returns SQL NULL when the field value is jbvNull: 'if (v != NULL && v->type != jbvNull) PG_RETURN_TEXT_P(...) else PG_RETURN_NULL()' — ->>' on JSON null yields SQL NULL, not 'null'. (https://github.com/postgres/postgres/blob/master/src/backend/utils/adt/jsonfuncs.c (jsonb_object_field_text, ~L902))
- **[research_citation]** sources.omitted: 5 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: PREDICATE (Q1): correct, verified at source level. Case table under (NOT (metadata ? 'product') OR metadata->>'product' = $X): [1] no product key -> NOT(false)=TRUE, TRUE OR NULL=TRUE (PG17 9.1 truth table) -> visible. [2] product=X -> FALSE OR TRUE -> visible. [3] product=Y -> FALSE OR FALSE -> excluded. [4] product: JSON null -> ? returns TRUE (key existence, Table 9.46; value not inspected), ->> returns SQL NULL for jbvNull (jsonfuncs.c jsonb_object_field_text: 'if (v != NULL && v->type != jbvNull) ... else PG_RETURN_NULL()'), so FALSE OR NULL = NULL -> not TRUE -> excluded, exactly the design stance. [5] non-string product (number/object) -> ->>' yields text serialization, never equals validated non-blank $X (coincidence like $X='5' harmless) -> excluded. [6] SQL-NULL metadata impossible: column NOT NULL (0001:33). No NULL-propagation trap: the NULL in case 4 is what produces the intended exclusion. INGEST BOUNDARY (Q2): VERDICT ACCEPTABLE, no strip. parse_wisdom raw-copies minus content/promoted_at/promotion_state (ingest.rs:133-138), so an ADV entry CAN carry product and it persists. But product is caller-domain filter data, not episode-owned state: contrast promotion_state, stripped because a forged value hides rows from default recall and poisons the transition state machine (spec 0011:42 rationale; store.rs:105-125 compare-and-set). Product has no episode writer, no state machine, no default exclusion. A forged product tag cannot cross the namespace barrier (namespace = ANY($1) ANDs independently, store.rs:37-43); within already-visible namespaces it grants nothing (untagged rows are visible anyway via the unscoped arm) and can only self-conceal from other product-scoped queries. Different in kind from promotion_state. Trust boundary unchanged: wisdom.jsonl is operator-controlled local input either way. IMPLEMENTATION SHAPE (Q3): confirmed. product must leave the shared map (store.rs:44-60 folds product+work_id+tags into one @>; kinds separate; namespace separate; two negative exclusions append). Single production path: build_recall_query store.rs:309 <- server.rs:130 passes client filters verbatim; only tests combine filters (recall_filters_it.rs:59-66, store.rs:489-496). Deliberate widening under combination (product+work_id now returns untagged work_id matches) is the intended AC1 semantics, scoped to product only per D1. The '?' literal is safe in sqlx-pushed SQL and already shipped (PROMOTE_SQL store.rs:110, MARK_CANDIDATES_SQL store.rs:125). INDEX (Q4): confirmed and slightly stronger than the design states: memories_metadata_gin is jsonb_path_ops (0002:1), which supports only @>, @?, @@ and NOT the key-exists operators (PG 8.14.4; GIN opclass table) — so even positive metadata ? 'product' is unservable, independent of the negation; ->>'= $X is an expression, unservable under any GIN. The whole OR is filtered-scan, same accepted shape as open-followup (store.rs:69-75, spec 0010:22) and promoted exclusion (store.rs:82-90, spec 0011:50); cost scales with corpus, bounded by HNSW ANN + LIMIT k, escalation path is recorded Option B. Irrelevant at 0 rows. PROBE PROTOCOL (Q5): sufficient for the user-approved closure bar (a): pinned baseline, defined artifacts, decision venue, binary outcome shape. BLAST RADIUS (Q6): design scope covers store.rs, tests, spec 0010. Additional surfaces found: (a) the byte-identical .adv/specs/0010 mirror exists ONLY in the main checkout and is untracked — absent from the worktree, so the cmp check and mirror edit must target /home/jon/dev/episode/.adv/specs/0010-recall-metadata-filters.md; (b) RecallFilters.product (types.rs:240) has no doc comment, so MCP JsonSchema exposes no semantics — add Product-OR-unscoped text; (c) unit test store.rs:488-512 survives the split unchanged (work_id/tags still produce @>) but should assert the new product arm; (d) README.md:24 is generic, no change; (e) spec 0010 line 9 AND line 22 both need rewrite (design D3 covers). LIMITATION: no shell/db tool in my surface — the live Postgres probe was not executable; semantics verified from official docs plus PostgreSQL C source instead.
- **[unresolved_action]** required_main_agent_actions: Correct design.md's broad GIN claim to state that the current jsonb_path_ops index does not support key existence; default jsonb_ops does.
- **[unresolved_action]** required_main_agent_actions: Record the four repository remediations on the change branch before acceptance.
- **[unresolved_action]** required_main_agent_actions: Do not revisit the Product-OR-unscoped predicate, JSON-null exclusion, caller-owned Product membership, or the accepted AC4 protocol-only completion bar without new evidence.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] Do not generalize a jsonb_path_ops limitation to all PostgreSQL GIN operator classes. Default jsonb_ops supports key-existence operators such as ?, while jsonb_path_ops supports @>, @?, and @@.
- **[archive_only_evidence]** changes_made: tests/recall_filters_it.rs: Added a composed Product, work_id, and kind assertion. It proves that an untagged matching row remains visible while wrong-Product and JSON-null rows remain excluded.
- **[archive_only_evidence]** changes_made: src/store.rs: Corrected the index comment to describe the current jsonb_path_ops index without making a false claim about every GIN operator class.
- **[archive_only_evidence]** changes_made: docs/specs/0010-recall-metadata-filters.md: Corrected the index statement: jsonb_path_ops lacks key existence support, and no indexed expression serves the extracted-text equality arm.
- **[archive_only_evidence]** changes_made: .adv/specs/0010-recall-metadata-filters.md: Applied the same index correction to keep both specification copies byte-identical.
- **[archive_only_evidence]** verification: tests_run=EPISODE_TEST_DATABASE_URL="postgres://episode:episode@localhost:5434/episode" cargo test --test recall_filters_it -- --ignored, cargo test --lib, EPISODE_TEST_DATABASE_URL="postgres://episode:episode@localhost:5434/episode" cargo test --tests -- --ignored, cargo clippy --all-targets -- -D warnings, cargo fmt --check, cmp ".adv/specs/0010-recall-metadata-filters.md" "docs/specs/0010-recall-metadata-filters.md", git diff --check results=pass — Targeted recall filters: 2 passed. Library: 63 passed. Ignored Postgres suites: 21 passed. Clippy had zero warnings. Format, mirror comparison, and diff checks passed. Product-only, Product plus work/kind, and no-filter paths executed successfully. The concord#46 protocol comment was posted at 2026-08-30T20:34:53Z after a zero-row baseline and before any outcome comment; it pins a 40-memory, two-project corpus threshold and transcript-based sufficient/insufficient cases.
- **[unresolved_action]** required_main_agent_actions: Keep the cleanup change in `tests/recall_filters_it.rs`; it prevents failed Product tests from contaminating the real probe corpus.
- **[unresolved_action]** required_main_agent_actions: Add `opencode export <sessionID> --sanitize` and a minimal extraction/redaction step to the concord#46 probe procedure before collecting evidence.
- **[unresolved_action]** required_main_agent_actions: Do not judge the probe yet: after cleanup the dev store has 7 rows across 3 namespaces and 0 Product-tagged rows, below the preregistered minimum.
- **[unresolved_action]** required_main_agent_actions: Treat 50,000 all-corpus rows with low Product selectivity, or a measured unacceptable p95, as the evidence point to reconsider deferred Option B.
- **[unresolved_action]** required_main_agent_actions: Restore the BRIEFING PACKET slice in future reviewer spawn packets.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] Filtered pgvector recall can choose different ranking plans by candidate selectivity. Namespace-scoped recall used the composite primary key plus exact sort, while an all-corpus Product filter used HNSW at high selectivity and switched to an exact sequential scan at 1% selectivity.
- **[wisdom_candidate]** wisdom_candidates: [convention] Database integration tests that default to a shared dev database must perform cleanup before assertions and after any returned test-body error. End-of-test cleanup alone leaks probe-contaminating rows when an assertion or recall fails.
- **[archive_only_evidence]** changes_made: tests/recall_filters_it.rs: Moved assertions after unconditional namespace cleanup so returned seed or recall errors cannot leave Product test rows in the default dev database. Removed five leaked rows from a prior failed run.
- **[archive_only_evidence]** verification: tests_run=EXPLAIN (ANALYZE, BUFFERS) on synthetic 1,000, 10,000, and 50,000-row scratch corpora, 100-run warm SQL benchmark at 50,000 rows, cargo fmt --check, EPISODE_TEST_DATABASE_URL=postgres://episode:episode@localhost:5434/episode_harden_product cargo test --test recall_filters_it product_scope_includes_untagged_shared_pool -- --ignored, cargo clippy --all-targets -- -D warnings results=pass — Operational-robustness scope. At 50,000 total rows and 1,000 selected namespace rows, base/Product warm recall averaged 8.49/8.31 ms. Namespace count/Product count averaged 0.415/0.521 ms, so the OR arm added about 0.105 ms and did not dominate 1024-d distance ranking. All-corpus high-selectivity Product recall kept HNSW at about 0.14 ms warm. At 1% selectivity it switched to exact sequential scan plus sort at 37.9 ms and returned all 8 requested hits. Scratch database was dropped. Dev database now has 7 real rows across 3 namespaces and 0 Product-tagged rows, below the probe's 40-row and 10-tag minimum.
- **[epic_terminal_note]** epic.membership: shapeEpisodeStructuredMemory · Add product scoping to recall (order 4)

## Contract / AC Coverage

No contract items.

## Unresolved Actions

- Correct design.md's broad GIN claim to state that the current jsonb_path_ops index does not support key existence; default jsonb_ops does.
- Record the four repository remediations on the change branch before acceptance.
- Do not revisit the Product-OR-unscoped predicate, JSON-null exclusion, caller-owned Product membership, or the accepted AC4 protocol-only completion bar without new evidence.
- Keep the cleanup change in `tests/recall_filters_it.rs`; it prevents failed Product tests from contaminating the real probe corpus.
- Add `opencode export <sessionID> --sanitize` and a minimal extraction/redaction step to the concord#46 probe procedure before collecting evidence.
- Do not judge the probe yet: after cleanup the dev store has 7 rows across 3 namespaces and 0 Product-tagged rows, below the preregistered minimum.
- Treat 50,000 all-corpus rows with low Product selectivity, or a measured unacceptable p95, as the evidence point to reconsider deferred Option B.
- Restore the BRIEFING PACKET slice in future reviewer spawn packets.
