# rq-episode-manual-forget01

## Statement

Given a memory outside the requested namespace or from a non-manual source, when deletion is requested, then no record is deleted.

## Rationale

Ingested memories from `.adv/wisdom.jsonl` and `.adv/reflections.jsonl` are the durable superset and must never be removed by a tool call. Only manually created memories may be deleted, and only within their own namespace.

## Authority

- `src/store.rs` implements `forget` with a SQL predicate on `namespace` and `source = 'manual'`.
- `tests/recall_it.rs` covers wrong namespace, ingested source, unknown id, and repeated deletion.

## Verification

- Integration test `forget_manual_enforces_namespace_and_manual_source` proves the predicate behavior.

## Constraints

- Deletion must not affect ingested memories.
