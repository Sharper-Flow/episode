# Archive Briefing Digest

**Change ID:** fixCrossNamespaceMemoryId
**Title:** Fix cross-namespace memory id collision
**Status:** archived
**Generated:** 2026-08-30T19:56:51.584Z

## Identity Anchors

- CHANGE
- STATUS
- TERMINAL_GATE_SUMMARY
- Origin: discovery

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

No Epic membership

## Durable Facts

Showing 22 of 22 durable facts.

- **[report_follow_up]** follow_ups: Advisory A1: drop redundant memories_namespace_idx in 0003 (or record named follow-up) — PK (namespace,id) leading column covers namespace-only predicates per PG 17 indexes-multicolumn.
- **[report_follow_up]** follow_ups: Advisory A3: scope bare-id fetch_one test helpers in tests/promotion_state_it.rs:128 and tests/reconcile_state_it.rs:106 by namespace before landing the two-namespace regression test.
- **[report_follow_up]** follow_ups: Advisory A2: update store.rs doc comments at :166 and :181 that still describe the single-column arbiter.
- **[report_follow_up]** follow_ups: Advisory A4: document in acceptance notes that un-namespaced recall may now return same-id hits from different namespaces (intended fix outcome).
- **[research_citation]** sources: sqlx Migrator source (run loop, checksum, pending-only apply): run_direct validates applied-migration checksums (VersionMismatch on edit) and executes only pending migrations; applied ones are skipped, so startup re-run is a no-op. (https://github.com/launchbadge/sqlx/blob/main/sqlx-core/src/migrate/migrator.rs)
- **[research_citation]** sources: sqlx Postgres migrate source (transactional apply): apply() wraps migration SQL plus bookkeeping INSERT in one transaction unless no_tx (comment cites launchbadge/sqlx#1966); advisory lock serializes concurrent migrators. (https://github.com/launchbadge/sqlx/blob/main/sqlx-postgres/src/migrate.rs)
- **[research_citation]** sources: sqlx docs.rs Migrator::run: Run pending migrations; validate previously applied migrations against current source to detect accidental changes. (https://docs.rs/sqlx/latest/sqlx/migrate/struct.Migrator.html)
- **[research_citation]** sources.omitted: 4 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: VERDICT: APPROVE WITH FINDINGS (no blocking corrections; five advisory notes).

1) ID-ADDRESSED QUERY INVENTORY — COMPLETE. Full production SQL inventory traced. Bare-id row addressing exists ONLY in the upsert conflict clause itself (src/store.rs:229). Every other path is already namespace-composite or namespace-scoped: promote WHERE id=$1 AND namespace=$2 (store.rs:105-108), forget_manual id+namespace+source (store.rs:336), exists_source / existing_source_ids namespace+source_id (store.rs:260, 288-289), forget_ingested + mark_candidates namespace+source_id (store.rs:362-364, 120-125), stats GROUP BY namespace (store.rs:442-445), recall returns id WITH namespace in RecallHit (store.rs:31, types.rs:318-321). MCP tools forget/promote already require namespace (server.rs:57-78), so the recall->promote/forget feedback loop is namespace-complete; no wire-schema change needed. No in-repo consumer dedups recall results by id. Test-only bare-id reads WHERE id=$1 ... fetch_one (tests/promotion_state_it.rs:128, tests/reconcile_state_it.rs:106) still work because test ids are uuid-unique; see advisory A3.

2) MIGRATION MECHANICS — CORRECT. Drop+add PK in one 0003 file is atomic: sqlx Postgres apply() executes migration SQL and the _sqlx_migrations bookkeeping INSERT in a single transaction unless the file is marked no-transaction (sqlx-postgres/src/migrate.rs, comment citing issue #1966); a mid-file failure rolls back fully and is not recorded applied. Re-run no-op (AC5) holds structurally: Migrator::run only executes PENDING migrations; applied ones are checksum-validated and skipped, so repeated startups never re-execute 0003 (migrator.rs run_direct). Consequence: never edit 0003 after it is applied anywhere — startup errors with VersionMismatch. Plain ALTER TABLE memories DROP CONSTRAINT memories_pkey (default {table}_pkey naming) is correct; IF EXISTS would only mask schema drift. Dependent objects: none — no FKs reference memories, and no Rust code names memories_pkey or memories_namespace_idx (grep verified). Extra safety the design can claim: the old global PK (id) strictly implies (namespace, id) uniqueness and both columns are already NOT NULL (0001_init.sql:9,13), so ADD PRIMARY KEY (namespace, id) cannot fail on existing data even if the zero-rows premise were stale.

3) ON CONFLICT RETARGET — SEMANTICS NARROW EXACTLY AS INTENDED. New arbiter (namespace, id) matches the new PK per unique-index inference (PG 17 sql-insert). The conflict set strictly NARROWS: old (id) fired on any-namespace same-id (the row-steal bug: bulk dedup sees namespace B clean because the row lives in A, lib.rs:210-228, then DO UPDATE SET namespace=EXCLUDED.namespace steals it, store.rs:229-232); new (namespace, id) fires only same-namespace, so cross-namespace same-id inserts a fresh row. No same-namespace conflict becomes newly reachable. The TOCTOU claim holds: filter_eligible drops stored (namespace, source_id)s before upsert (lib.rs:105-113, 210-228), and id==source_id for both parsers (ingest.rs:141-142, 268-271), so the DO UPDATE branch is reachable only via a concurrent writer between the dedup SELECT and the INSERT — exactly the race whose promotion_state-preserving CASE (store.rs:235-247) the harden fix added, and which spec 0011 line 43 documents. That CASE is untouched by the retarget. Dropping namespace = EXCLUDED.namespace from SET is correct: with arbiter (namespace,id) the matched row's namespace equals EXCLUDED.namespace by definition, so the assignment was the steal mechanism and is now dead code. Pre-existing non-blocking hazard unchanged: duplicate ids WITHIN one batch statement are not deduped by filter_eligible and raise the PG cardinality-violation error (cannot affect one row twice) under both old and new arbiter.

4) memories_source_uniq RETENTION — JUSTIFIED, RATIONALE IMPROVABLE. It is the write-side enforcer of the exact key the dedup READ uses — (namespace, source, source_id), cf. existing_source_ids store.rs:288-289 — while the PK covers that key only through the parsers' private id==source_id convention, which no schema constraint enforces. It can independently fire only if id ever diverges from source_id (same ns+source+source_id, different id), which is precisely the divergence case. Cost is one partial index over zero rows. Retain. The stated rationale ('guards if id diverges') is right but should be phrased as dedup-read/write key alignment.

5) REJECTED ALTERNATIVE — REJECTION UPHELD. Namespace-qualified stored ids ({ns}/pw-1) keep the single-column PK but denormalize namespace into the id, invent a wire format, and change every caller-visible ingested id plus every existing_source_ids comparison. Composite PK concerns checked and dismissed: pgvector HNSW index is independent of the PK (0001:50-51); no replication configured (docker-compose.yml — single pg16 node); PK width (2x TEXT) negligible at this scale; composite PK is the documented PG form (ddl-constraints PRIMARY KEY (a,c)).

6) BLAST RADIUS BEYOND THE DESIGN LIST — THREE ITEMS. (a) memories_namespace_idx (0001:54) becomes REDUNDANT: the PK btree (namespace, id) has namespace leading and serves all namespace-only predicates (indexes-multicolumn), and no code references the index by name — candidate for DROP in 0003 (P41). (b) Stale doc comments referencing the old arbiter must be updated with the code (store.rs:166 'keyed on stable id', store.rs:181 'ON CONFLICT (id)'). (c) Wire-visible behavior: un-namespaced recall may now legitimately return two hits with the same id in different namespaces — previously impossible because the bug collapsed them; correct by design, worth one line in acceptance notes. No durable doc restates the single-column PK as a contract (grep: only read-only .adv/archive artifacts).
- **[unresolved_action]** required_main_agent_actions: Checkpoint the three reviewer-modified files before acceptance.
- **[unresolved_action]** required_main_agent_actions: Keep migrations/0003_namespace_id_pk.sql byte-identical because the development database has already applied it.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] Never edit an applied sqlx migration, including comments. sqlx validates the full migration checksum and rejects any changed byte on the next connection.
- **[archive_only_evidence]** changes_made: tests/reconcile_state_it.rs: Added explicit unchanged-source no-op coverage, reused one raw id in the retraction scope test, and removed stale global-id commentary.
- **[archive_only_evidence]** changes_made: tests/recall_it.rs: Added same-id manual rows in two namespaces and proved forget removes only the addressed namespace row. Removed stale contract citations from the test comment.
- **[archive_only_evidence]** changes_made: src/store.rs: Removed stale agreement and design citations from the upsert documentation.
- **[archive_only_evidence]** verification: tests_run=cargo test --test reconcile_state_it -- --ignored, cargo test --test recall_it -- --ignored, EPISODE_TEST_DATABASE_URL=postgres://episode:episode@localhost:5434/<isolated-db> cargo test --tests -- --ignored, cargo test --lib, cargo clippy --all-targets -- -D warnings, cargo fmt --check, git diff --check results=pass — Final results: 7/7 reconcile tests, 2/2 recall tests, 20/20 DB-backed tests on a fresh isolated database, and 63/63 library tests passed. Clippy reported zero warnings. Formatting and diff checks passed. The fresh database exercised migrations 0001-0003; repeated Store::connect calls proved migration reruns are no-ops. The isolated database was dropped by an EXIT trap. An initial targeted run failed after a review-only comment edit changed migration 0003's checksum. I reverted that edit byte-for-byte, confirmed no migration diff from HEAD, and reran all checks successfully. Persisted execution evidence also records AC7's pre-0003 failure and post-0003 pass.
- **[unresolved_action]** required_main_agent_actions: Keep the README stop, backup, and schema-compatible rollback guidance with this change.
- **[unresolved_action]** required_main_agent_actions: For the stated zero-row service restoration, retain the plain transactional migration and continue hardening.
- **[unresolved_action]** required_main_agent_actions: If a future database reaches roughly one million rows or multiple gigabytes, benchmark the migration and consider a staged concurrent unique-index build with PRIMARY KEY USING INDEX.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] A forward schema migration can be transactionally safe yet make binary-only rollback unsafe. Document the binary/schema compatibility boundary and require a matching database snapshot for rollback.
- **[archive_only_evidence]** changes_made: README.md: Added durable deployment guidance to stop the service, take a backup, and restore a schema-compatible snapshot for binary rollback.
- **[archive_only_evidence]** verification: tests_run=EXPLAIN six production query shapes on a 20,002-row, 102-namespace, 242 MB scratch database, Timed migration 0003 transaction on the populated scratch database, Started two Episode processes simultaneously against migration 0002, Ran an old ON CONFLICT (id) upsert while migration 0003 held its table lock, cargo fmt --all --check, cargo clippy --all-targets --all-features --locked -- -D warnings, cargo test --locked, EPISODE_TEST_DATABASE_URL='postgres://episode:episode@localhost:5434/episode' cargo test --tests -- --ignored, git diff --check results=pass — Operational-robustness area 1 is clean at expected scale: the populated migration was atomic and took 0.22 seconds locally; ACCESS EXCLUSIVE and synchronous rebuild become material at million-row or multi-GB scale. Area 2 is clean: selective namespace query shapes used memories_pkey or memories_source_uniq; stats correctly used a full-table Seq Scan. Area 3 is clean for new processes: SQLx locking serialized starts. An old-service upsert waited about 2 seconds for DDL, then failed because ON CONFLICT (id) no longer had a matching unique constraint, which confirms the stop-first cutover requirement. Area 4 is clean: failure rolls back the file transaction; restore of a pre-0003 backup lets the new binary replay 0003, while old-binary rollback requires the matching snapshot. Area 5 is clean for this deployment: startup propagates migration errors before serving, the service-down cutover avoids pool lock waits, DROP INDEX removed the relation without VACUUM, and dev has no replication. Large replicated deployments can see WAL and replay lag from the index rebuild.

## Contract / AC Coverage

No contract items.

## Unresolved Actions

- Checkpoint the three reviewer-modified files before acceptance.
- Keep migrations/0003_namespace_id_pk.sql byte-identical because the development database has already applied it.
- Keep the README stop, backup, and schema-compatible rollback guidance with this change.
- For the stated zero-row service restoration, retain the plain transactional migration and continue hardening.
- If a future database reaches roughly one million rows or multiple gigabytes, benchmark the migration and consider a staged concurrent unique-index build with PRIMARY KEY USING INDEX.
