# Design — promotion state

## The root cause behind both defects

Episode's ingest is **write-once per `source_id`**. `reconcile_root` (`src/lib.rs:159-255`) reads `existing_source_ids` (`:183`), then `filter_eligible` (`:105-113`, called at `:199`) drops every item whose `source_id` is already stored. `upsert_batch` therefore only ever sees new rows.

Consequence: **episode cannot observe any post-ingest mutation of ADV state.** Both fields this change cares about are exactly that.

| ADV field | Mutated when | Episode today |
|---|---|---|
| `invalidated_by` | after a lesson is retracted | row stays stored and keeps surfacing |
| `promoted_at` | after a lesson graduates | never mapped — dedup skips the row |

The parse-time skip at `src/ingest.rs:67-70` prevents *adding* an invalidated entry but cannot remove a stored one. And a `promoted_at` mapping in `parse_wisdom` would be inert in the common case, because wisdom is overwhelmingly ingested before it is promoted.

These are one missing mechanism, not two bugs. Reconcile needs a step that applies post-ingest state changes to rows that already exist.

## Decisions

### D1. `target` stays opaque

`Promoted { target: String }`. Episode stores what the caller supplies and never parses it.

Concord's contract resolves a target by manifest path plus sha256, names a record version rather than a moving document, publishes **no serialized format**, and states `target` is opaque to episode. Concord also exposes no runtime resolution surface, so episode could not verify resolvability even in principle. Validation beyond non-blank would be theater.

**Accepted failure mode, recorded rather than mitigated away.** A malformed target is excluded from episode recall *and* rejected by Concord as `knowledge_missing` — silent loss on both sides. The two-step flow is the mitigation: ADV promotion lands a memory in `PromotionCandidate`, which stays visible in recall, and only an explicit human `promote` carrying a real target moves it to `Promoted`. Nothing is hidden until a human supplies the target. The demote path below is the recovery.

### D2. Invalidated entries are removed, not marked

Ingest collects source ids carrying a non-null `invalidated_by` and deletes the corresponding stored rows.

Delete-over-mark is self-healing: if an entry is later un-invalidated in ADV, the row is absent from `existing_source_ids`, so the next reconcile re-ingests it fresh.

**Consequence: `Superseded { by }` is dropped.** No producer exists for it. Manual supersession is served by `forget_manual`, and `parse_reflections` has no `invalidated_by` check at all, so reflections cannot produce it either. A state no caller can create is dead on arrival.

The enum carries two stored variants instead of three, and default recall exclusion needs one predicate instead of two.

### D3. Transitions are bidirectional and not source-restricted

`Store::promote` borrows `forget_manual`'s **shape** — one statement, structural predicates, rows-affected return — but **not its `source = 'manual'` restriction**. The row a human most needs to promote is an ingested wisdom entry carrying `promoted_at`. Restricting to manual rows would make the primary use case impossible.

Transitions set any target state under a per-transition precondition asserted in SQL. This yields demotion for free, which is the recovery path for D1's accepted failure mode: a `Promoted` row whose Concord target is later deleted would otherwise be excluded from recall permanently, with no way back.

One `UPDATE` carrying its from-state predicate is safe under READ COMMITTED. A concurrent loser blocks on the row lock, re-evaluates, and returns zero rows affected. This holds only while the JSONB merge stays in SQL — never read-modify-write in Rust.

## Type

Following `ActionState` (`src/types.rs:57-67`), the house precedent for a tagged state round-tripping through MCP params into JSONB:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
pub enum PromotionState {
    PromotionCandidate {},
    Promoted { target: String },
}
```

`#[schemars(crate = "rmcp::schemars")]` is required because rmcp re-exports schemars. The empty-payload variant matches `LinkedWork {}` / `OpenFollowup {}` (`src/types.rs:64-66`), already proven closed under schemars by the schema test at `src/server.rs:489-499`.

**`Episodic` is absent, not stored.** It is the default, and therefore the absence of a `promotion_state` key. An explicit marker would force a backfill across every row and buy nothing, since the predicate excludes one tagged shape and passes everything else. This follows the open-followup precedent.

Validation mirrors `MemoryContext::validate` (`src/types.rs:113-128`): `Promoted` requires a non-blank `target`.

## Storage

JSONB under reserved key `promotion_state`, alongside the existing `action` key. No migration, no new column — matching the `action` precedent at `src/types.rs:109`.

```json
{"promotion_state": {"kind": "promoted", "target": "..."}}
```

## Recall filtering

Mirrors `include_open_followups` (`src/store.rs:66-72`). New `RecallFilters` bool defaulting to `false`, adding one predicate when unset:

```sql
NOT (metadata @> '{"promotion_state":{"kind":"promoted"}}')
```

`PromotionCandidate` stays visible by default, deliberately. A candidate has not graduated, so episode still holds the authoritative copy.

**GIN caveat, stated not assumed.** `docs/specs/0010-recall-metadata-filters.md:22` records that the index claim covers positive containment only. This negative predicate is unindexed, exactly as the open-followup exclusion already is. Filtered-scan cost scales with the *excluded* fraction, which human promotion activity bounds. The new spec must say this.

Adding the field is backward-compatible: `RecallFilters` is `#[serde(default)]` (`src/types.rs:131-132`) and is never persisted, so an omitted flag yields `false` and current behavior.

## Ingest — post-ingest state reconciliation

`parse_wisdom` (`src/ingest.rs:57-104`) additionally returns collected state changes rather than only skipping:

- entries with non-null `invalidated_by` → collect id for **deletion**
- entries with non-null `promoted_at` → collect id for **transition to `PromotionCandidate`**

`reconcile_root` gains a step after `upsert_batch` that applies these to already-stored rows, outside the dedup path:

| Change | Store method | Effect |
|---|---|---|
| invalidated | `forget_ingested` | delete rows by `source_id` within namespace, restricted to ingested sources |
| promoted | `mark_candidates` | set `promotion_state` to `promotion_candidate` by `source_id`, only where the key is currently absent |

The `where currently absent` guard makes this idempotent and stops reconcile from clobbering a human `Promoted { target }` back down to a candidate on every pass.

For entries arriving already-promoted on first ingest, `parse_wisdom` also writes the state inline after the metadata clone at `src/ingest.rs:87`, and **removes `promoted_at`** so the two keys cannot diverge.

### Why ingested promotions become `PromotionCandidate`, not `Promoted`

ADV's `promoted_at` is a timestamp. ADV wisdom carries no manifest path and no sha256, so an ingested promotion has **no Concord target to record**.

`Promoted { target: Option<String> }` would assert graduation and trigger recall exclusion while pointing at nothing — the memory would vanish from episode and be unreceivable at Concord simultaneously. Mapping to `PromotionCandidate` is faithful: ADV says this graduated, episode keeps serving it, and a human supplies the Concord target through `promote` when the record exists.

This narrows AC6 rather than satisfying it as originally worded. AC6 is amended accordingly.

## Surfaces

| Surface | Shape |
|---|---|
| `promote` tool | Mirrors `ForgetParams` (`src/server.rs:54-62`): id, namespace, target state. Not source-restricted. |
| `Store::promote` | One statement, from-state precondition in SQL, JSONB merge in SQL. Bidirectional. |
| `Store::forget_ingested` | Delete by `source_id`, ingested sources only. |
| `Store::mark_candidates` | Set candidate state by `source_id`, only where `promotion_state` is absent. |
| `RecallFilters` | One new bool, default false. |

## Verification

- DB-free SQL test paralleling `src/store.rs:332-356`: the new predicate appears, and state literals never enter SQL syntax.
- Integration test paralleling `tests/recall_filters_it.rs:64-87`: seed promoted and unpromoted rows, assert default exclusion and opt-in inclusion.
- **Regression test for the retraction defect** — ingest an entry, invalidate it in ADV, reconcile, assert the row is gone. Fails today.
- **Regression test for the inert-mapping defect** — ingest a valid entry, set `promoted_at` in ADV, reconcile, assert the row is now `PromotionCandidate`. Fails under a parse-only design, and a `parse_wisdom` unit test cannot catch it. This must be store-level.
- Idempotency test — human sets `Promoted { target }`, then reconcile runs again, assert the state is not clobbered back to candidate.
- Transition tests — each valid transition, demotion, and a from-state mismatch returning zero rows affected.
- Ingest unit test paralleling `src/ingest.rs:264-316`: inline `promoted_at` maps and is removed from metadata.
- Schema test paralleling `src/server.rs:447-506`: variant tags.

## Spec

`0011-promotion-state.md`, written verbatim to both `docs/specs/` and `.adv/specs/`, per the convention 0010 follows.

## Scope note

Both defect fixes are folded in rather than deferred, because they share one root cause with the feature: episode cannot observe post-ingest ADV mutations. Fixing that is what makes promotion state work at all — without it the ingest mapping is inert and retracted knowledge persists.

Net effect is still subtractive against the original proposal: one fewer enum variant, one fewer recall predicate, one fewer stale-data failure mode.

**Blast radius is latent, not active.** The store holds zero rows and the service has been disabled since 2026-08-19. Neither defect is harming anyone today. Both become live on the first reconcile after the service is restored, which is precisely why they should land before it comes back up.
