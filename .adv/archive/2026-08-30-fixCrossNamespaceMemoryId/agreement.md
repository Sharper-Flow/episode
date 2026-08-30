# Agreement — fix cross-namespace memory id collision

## Objectives

1. Two projects ingesting the same raw ADV id (`pw-1`) both retain independent rows. Neither reconcile steals the other's row.
2. The fix is structural: the `ON CONFLICT` target aligns with whichever uniqueness constraint is authoritative, rather than sidestepping collisions.
3. No caller-facing contract change: `remember`, `recall`, `forget`, and `promote` keep their current parameter shapes and semantics.
4. The distinct-id fixture workaround in `tests/reconcile_state_it.rs` becomes provably unnecessary — real isolation is demonstrated by test, not by id avoidance.

## Acceptance criteria

- AC1: Ingesting the same ADV id under two distinct namespaces produces two surviving rows, each addressable and recallable in its own namespace.
- AC2: Reconcile of project B does not modify, move, or delete any row belonging to project A (asserted on content, namespace, and promotion state).
- AC3: Re-ingest of an unchanged source under the same namespace is still a no-op (dedup via `existing_source_ids` continues to work; `upsert_batch`'s conflict path is only the concurrency race).
- AC4: `promote` and `forget` address rows by id + namespace exactly as before; a same-id row in another namespace is untouched.
- AC5: A migration exists, is forward-only, and runs on an empty store; re-running migrations is a no-op.
- AC6: Manual memories (`mem-{uuid}`) are unaffected.
- AC7: The regression test (AC1/AC2) fails on the pre-fix schema and passes after — proven by revert check, not assumed.
- AC8: `cargo test --lib`, all DB-backed tests, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass.

## Constraints

- Episode service is down and the store holds zero rows. The migration may reshape the primary key without backfill; it must not depend on row data.
- No speculative unique-index cleanup: `memories_source_uniq` stays. It guards a distinct constraint (same source_id under different id shapes) and is not dead.
- Promotion-state semantics from spec 0011 are untouched; the promotion tests must pass unchanged.

## Avoidances

- Do not namespace-qualify the stored id (rejected direction): it denormalizes namespace into id, invents a wire format, and changes every caller-visible ingested id.
- Do not alter MCP tool schemas.
- Do not fold in unrelated store refactors.