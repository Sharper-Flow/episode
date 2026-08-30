# Fix cross-namespace memory id collision

> Routed finding. Surfaced while writing a namespace-scoping test in
> `addPromotionStateMemories` task `tk-76bb0834b5e5`. Pre-existing and unrelated
> to promotion state, so it was not folded into that change.

## Problem

Ingested memories use the raw ADV entry id as the global primary key. ADV wisdom
ids are per-project sequential (`pw-1`, `pw-2`, ...), so two watched projects
almost certainly both contain `pw-1`. The second project's reconcile silently
steals the first project's row.

Evidence:

- `migrations/0001_init.sql:9` — `id TEXT PRIMARY KEY`, global across namespaces.
- `src/ingest.rs` — `parse_wisdom` sets `id` to the raw ADV id, unprefixed.
- `src/store.rs:211-221` — `ON CONFLICT (id) DO UPDATE SET namespace = EXCLUDED.namespace, ...`.

The intended guard, `memories_source_uniq` on `(namespace, source, source_id)`
(`migrations/0001_init.sql:45-47`), never fires: the primary-key conflict is
evaluated first, so the statement takes the `DO UPDATE` branch and rewrites the
existing row's namespace instead of inserting a second row.

## Observed, not just derived

This is not a paper reading of the schema. It reproduced within minutes of being
found.

While implementing `addPromotionStateMemories` task `tk-568afae7fe07`, two new
integration tests in `tests/reconcile_state_it.rs` each seeded an entry with id
`pw-1` under two different, uniquely-named namespaces. They collided. The second
test's upsert took the `ON CONFLICT (id) DO UPDATE` branch and moved the first
test's row into the second namespace, so a state assertion read `None` where the
row should have been.

The failure presented as a promotion-state bug and was not one. That is the
shape this defect will take in production: a memory goes missing in project A,
and the cause looks like whatever subsystem happened to read it next.

The tests were fixed by giving each a distinct id. That is a fixture workaround,
not a fix — it sidesteps the collision rather than resolving it.

## Impact

Project A ingests `pw-1`. Project B later ingests its own unrelated `pw-1`. A's
memory is overwritten in place — content, kind, metadata, embedding, and
namespace all replaced. A loses the memory with no error and no log line. Recall
for A silently returns less than it should.

Manual memories are unaffected: they use `mem-{uuid}` ids.

Reflection children are also exposed — `{rf_id}:{kind}:{index}` is likewise
per-project.

## Severity

Latent today and cheap to fix now. The store holds zero rows and the service has
been disabled since 2026-08-19. It becomes live as soon as episode watches a
second project after restoration.

## Scope

- `migrations/` — one new migration, shape depending on the direction chosen.
- `src/ingest.rs` — `parse_wisdom` and `parse_reflections` id construction.
- `src/store.rs` — `upsert_batch`'s `ON CONFLICT` target; check `forget_manual`,
  `forget_ingested`, `mark_candidates`, and `promote`, which all address rows by
  `id` plus `namespace` and are correct either way.
- `src/server.rs` — `forget` and `promote` take a caller-supplied `id`; confirm
  whether the caller-facing id changes shape.
- `tests/reconcile_state_it.rs` — remove the distinct-id workaround once real
  isolation exists.

## Candidate directions

Not yet decided; discovery should settle it.

1. Namespace-qualify the stored id at parse time. Keeps `source_id` as the raw
   ADV id for dedup, so `existing_source_ids` is unaffected. Changes the
   caller-facing id, which `forget` and `promote` accept.
2. Make the primary key composite on `(namespace, id)` and retarget the
   `ON CONFLICT` clause. Larger migration, no id rewriting, caller-facing id
   unchanged.

Either way the fix must keep the `ON CONFLICT` target aligned with whichever
uniqueness constraint is authoritative, since that misalignment is the bug.

## Verification

A regression test must ingest the same ADV id under two namespaces and assert
both rows survive independently. Such a test cannot pass today.
