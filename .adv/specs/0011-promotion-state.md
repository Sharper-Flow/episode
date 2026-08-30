# rq-episode-promotion-state01

## Statement

A memory carries a promotion state recording its position on the path from episodic recall to durable Product knowledge, and recall stops serving memories a Concord record now owns.

## State Contract

- `promotion_candidate` marks a memory as graduated in ADV or recurring in practice. Episode still holds the authoritative copy.
- `promoted` marks a memory as owned by a Concord `spec` or `decision`, and carries the `target` naming it.
- Episodic is the **absence** of the `promotion_state` metadata key, never a stored value. No backfill exists or is needed.
- State lives under the reserved `metadata.promotion_state` key beside `action`. No typed column, no migration.
- `superseded` does not exist. Retracted memories are deleted rather than marked, and manual deletion is served by `forget`, so no caller could produce it.

## Target Opacity

`target` is stored verbatim and validated only as non-blank. Concord resolves a target by manifest path plus sha256, publishes no serialized format, and exposes no runtime resolution surface, so episode cannot verify a target even in principle. Any deeper local check would assert knowledge episode does not have.

The accepted consequence: an unresolvable target excludes the memory from episode recall while Concord rejects it as `knowledge_missing`, losing it on both sides at once. Two mechanisms bound this. Nothing is hidden until a human supplies a target, because ADV-recorded graduation produces a candidate and candidates stay visible. And demotion returns an over-promoted memory to visibility.

## Recall Contract

Default recall excludes `metadata.promotion_state.kind = promoted` while retaining episodic and candidate memories. `include_promoted: true` includes graduated rows that satisfy all remaining filters. The flag defaults to `false`, so an omitted field stops serving superseded knowledge without any caller change.

Candidates stay visible in both modes: a candidate has not graduated, so episode remains its only source.

## Transition Contract

- Transitions run in both directions. Demotion is the recovery path when a Concord target is deleted; without it a promoted memory would be excluded from recall permanently.
- A transition asserts the state the memory is expected to be in. A mismatch, a wrong namespace, or an unknown id changes nothing and reports zero rows updated.
- Transitions are **not** restricted by source. Ingested memories are promotable, and are the primary case: the row a human most needs to graduate came from ADV wisdom.
- The precondition and the merge are one statement. A concurrent loser blocks on the row lock, re-evaluates against the committed row, and matches nothing.

## Ingest Contract

Ingestion is write-once per `source_id`: already-stored items are dropped before persistence. ADV mutates both fields below *after* first ingest, so reconcile applies them to stored rows directly rather than through the ingestion path.

- `invalidated_by` non-null removes the stored row, restricted to ingested sources. Deletion is self-healing: un-invalidating in ADV restores the memory on the next pass, because an absent row re-ingests fresh. Retraction is applied before ingestion so a later embedding failure cannot leave withdrawn knowledge in service.
- `promoted_at` non-null sets `promotion_candidate`, and only where no promotion state exists. That guard makes the operation idempotent and prevents a later reconcile from resetting a human-set `promoted` back to a candidate, since `promoted_at` remains in the source file permanently.
- Ingested promotions become candidates, never `promoted`. ADV records graduation as a timestamp and carries no manifest path or sha256, so no Concord target exists to name.
- The raw `promoted_at` is removed from stored metadata. Retaining it would expose a second promotion field populated only on entries that arrived already-promoted, because dedup freezes metadata at first write.
- Both operations are scoped to the reconciled namespace and to ingested sources, so reconcile can never delete or alter a manual memory.

## Query and Index Safety

All state values are bound with sqlx; no state literal enters SQL syntax. The transition precondition compares with `IS NOT DISTINCT FROM` so a NULL parameter matches an absent key, letting one statement cover every transition including the episodic start state.

The default exclusion is a **negative** predicate and does not use `memories_metadata_gin`, exactly as the negative open-follow-up predicate does not. See `docs/specs/0010-recall-metadata-filters.md`, which records that the index claim covers positive containment only. Filtered-scan cost scales with the excluded fraction, which human promotion activity bounds.

## Boundaries

Automatic promotion heuristics remain separate; promotion is explicit. This capability adds no Concord-side behavior — the receiving contract is consumed, not modified. It does not change recency, ranking, ingest quality, typed columns, or Fleet behavior.

## Verification

DB-free tests prove variant tags, schema closedness, target validation, bound SQL structure, and the absence of a source restriction on transitions. Model-free Postgres integration proves default exclusion, opt-in inclusion, candidate visibility, every transition including demotion and mismatch, promotion of ingested rows, removal of retracted rows and its namespace scoping, mapping of graduation recorded after first ingest, idempotency across repeated reconciles, and removal of the raw `promoted_at` field.
