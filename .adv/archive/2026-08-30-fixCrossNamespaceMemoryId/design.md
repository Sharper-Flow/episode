# Design — composite primary key (namespace, id)

## Root cause

`migrations/0001_init.sql:9` declares `id TEXT PRIMARY KEY` — globally unique across namespaces. Ingested ids are raw per-project ADV ids (`pw-1`, `{rf_id}:{kind}:{index}`), so a second project's reconcile hits the PK conflict and takes `upsert_batch`'s `ON CONFLICT (id) DO UPDATE SET namespace = EXCLUDED.namespace, ...` branch (`src/store.rs:229`), silently rewriting the first project's row in place. The intended guard, `memories_source_uniq (namespace, source, source_id)` at `:45-47`, never evaluates — the PK conflict wins first.

The row's real identity domain is (namespace, id). Every id-addressed query already assumes it: `promote` (`src/store.rs:105-108`) and `forget_manual` (`:336`) both filter `id = $1 AND namespace = $2`; `forget_ingested`/`mark_candidates` bind namespace + source ids; `RecallHit` returns id **and** namespace; the MCP tools already require namespace. Only the primary key disagreed.

## Decision D1 — composite PK, not id rewriting

**Chosen:** drop `memories_pkey`, add `PRIMARY KEY (namespace, id)` in migration `0003_namespace_id_pk.sql`. Retarget `upsert_batch`'s conflict clause to `ON CONFLICT (namespace, id)`.

**Rejected:** namespace-qualifying the stored id at parse time (`{namespace}/pw-1`). It would keep the single-column PK but denormalize namespace into every ingested id, invent a separator convention that becomes caller-visible wire format, and change the id shape for wisdom rows, reflection children, and every stored reference. Identity belongs in the constraint, not copied into the value.

**Cost:** zero-row store (service down since 2026-08-19) makes the PK rebuild an instant catalog operation. No backfill, no dual-write, no compatibility window. Independent validator confirmed the zero-rows premise is not even load-bearing: global `(id)` uniqueness strictly implies `(namespace, id)` uniqueness, so the migration cannot fail on data at any row count.

## What changes

1. **`migrations/0003_namespace_id_pk.sql`** — plain `ALTER TABLE memories DROP CONSTRAINT memories_pkey` then `ADD PRIMARY KEY (namespace, id)`, plus `DROP INDEX memories_namespace_idx` (validator A1: the composite PK's btree leads on namespace and serves every namespace-only predicate, making the standalone index redundant). No `IF EXISTS` guards: sqlx applies each migration exactly once in its own transaction with checksum validation (validator-verified against sqlx source), so idempotency comes from the migrator, not the SQL — and the file is immutable once applied.
2. **`src/store.rs` `upsert_batch`** — conflict target `(namespace, id)`; drop `namespace = EXCLUDED.namespace` from the `DO UPDATE` set list (a row conflicting on `(namespace, id)` cannot change namespace; keeping the assignment is misleading). The promotion-state preservation branch from `addPromotionStateMemories` keeps its exact semantics — it guards metadata regardless of which conflict target fired.
3. **Stale doc comments** (`src/store.rs:166,181` — validator A2) — rewritten to the new conflict target under comment hygiene: current behavior only, no change narration.
4. **Test helpers** (`tests/promotion_state_it.rs:128`, `tests/reconcile_state_it.rs:106` — validator A3) — bare-id `fetch_one` helpers gain a namespace bind before the two-namespace regression test lands, because a bare id can now legitimately match two rows.
5. **`tests/reconcile_state_it.rs`** — new regression `same_adv_id_in_two_namespaces_survives_independently`: ingest `pw-1` under namespaces A and B, reconcile both, assert both rows survive with their own content; promote B's row and assert A's is untouched. Existing fixture distinct-ids (`pw-late`, `pw-idem`, ...) stay — renaming them adds churn without adding proof the new test does not carry.

## What deliberately does not change

- `promote`, `forget`, `forget_ingested`, `mark_candidates`, `existing_source_ids`, stats: all already namespace-scoped (validator-verified inventory; no bare-id row addressing exists outside the old conflict clause).
- MCP tool schemas and `RecallHit` shape: no caller-contract change. One behavior note for acceptance (validator A4): recall across multiple namespaces may now return same-id hits from different namespaces — the intended outcome of the fix, since those are distinct memories.
- `memories_source_uniq`: retained. Validator sharpened the rationale: it is the write-side enforcer of the dedup **read** key `(namespace, source, source_id)` (`store.rs:288-289`), which the PK covers only through the unenforced parser convention that id == source_id. Dropping it would leave dedup correctness resting on a convention instead of a constraint.
- HNSW and GIN indexes: reference no PK column.

## Concurrency note

The conflict set strictly narrows. Old `(id)` fired on any-namespace same-id — the steal. New `(namespace, id)` fires only on same-namespace re-ingest, which is exactly the TOCTOU race `filter_eligible` cannot close (check-then-act across two reconciles of one namespace). Cross-namespace collision is structurally impossible: two namespaces cannot produce the same `(namespace, id)` pair.

## Verification strategy

RED: the new regression test against the current schema — namespace B's reconcile steals A's row and the first assertion fails. GREEN: migration + conflict retarget. Revert-proof (AC7): run the regression against a database migrated only through 0002 and confirm the steal, then through 0003 and confirm isolation. Full gate: lib tests, all DB-backed suites, clippy `-D warnings`, fmt, and a migration re-run against an already-migrated database (must be a no-op via sqlx bookkeeping, per validator source-check).

## Independent validation

adv-researcher verdict: **APPROVE WITH FINDINGS** — pass, high confidence, low risk, zero blocking corrections. Five advisory notes (A1 redundant index, A2 stale comments, A3 test-helper scoping, A4 recall behavior note, A5 migration immutability/plain DROP) all folded above. Sources: sqlx migrate implementation (transactional single-apply, checksum validation), PostgreSQL 17 docs (conflict-target inference, composite PK auto-index, leading-column rule, ALTER TABLE PK swap).