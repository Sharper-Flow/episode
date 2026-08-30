# Add product scoping to recall

## Problem

Current namespace model is flat per-project (`pokeedge`, `advance`, `global`). Concord's model is **Product to Project to work**. An agent working within a Product context needs recall scoped to "all projects under this Product + global" without the caller manually unioning namespaces.

This is the probe surface for Concord's C20 reopen trigger: it measures whether external product-scoping is sufficient for real multi-project Product work.

## Finding (2026-08-30): the shipped filter does not implement the required semantics

`RecallFilters.product` exists and reaches SQL as a positive containment `metadata @> {"product": X}` (`src/store.rs:46-48,55-60`). Positive containment excludes every row without a `product` key — including global and untagged ingested rows. AC1 requires "that Product's memories **+ global**". The required predicate is **Product-OR-unscoped**: `(product = X) OR (product absent)`.

Composition check: the namespace constraint stays independent. An advance-tagged global row still fails a pokeedge-scoped query (product present, wrong value); an untagged row in a passed namespace stays visible (ingested wisdom of in-scope projects); an untagged global row stays visible (the "+ global"). Semantics hold under all three cases.

Index note: one arm of the OR is negative (key absence), which the GIN index does not serve — same accepted shape as the open-followup and promoted exclusions (spec 0010 line 22, spec 0011 "Query and Index Safety").

## Proposed change

### Option A (chosen): fix the filter semantics; no schema change

1. Product filter becomes Product-OR-unscoped in `build_recall_query`.
2. Product tags arrive caller-side only: `MemoryContext.product` on `remember` (shipped, Tier 1a). Episode never owns the Product-to-namespace mapping — the caller (Concord orchestration in a Product context) supplies it, per the caller contract below.
3. Untagged rows are global by nature: predecessor wisdom ingested from ADV carries no product and composes correctly with the namespace constraint. No ingest-side derivation, no operator config mapping — deriving membership would put Concord's Product model inside episode, which AC2 forbids.
4. Probe protocol and baseline recorded against concord#46 (AC4).

### Option B (deferred): promote product to a typed column + partial index

Migration would be `0004` (0003 is taken by the composite PK). Promote only if product-scoped recall becomes a hot path with measured latency pain. Not justified now: zero rows in the store.

## Caller contract

Episode never owns the Product-to-namespace mapping. The caller resolves "this Product's projects" and passes `namespaces` (the Product's projects + `global`) plus `filters: { product: "..." }`. Episode stays Concord-agnostic.

## Probe design

The probe runs against real multi-project Product work after this change ships and the corpus grows. The evidence AC4 requires: concrete recall transcripts from Product-scoped sessions, what was returned vs. what was needed, and whether namespace+product filtering expressed the working context or fought it. Outcome recorded against concord#46 as sufficient/insufficient with the transcripts as evidence — not a vibe.

**Completion bar (open question for the user):** AC4 can mean (a) this change delivers the probe protocol + baseline state and closes, with the outcome recorded later as a follow-up when real evidence exists; or (b) this change stays open until the real-usage outcome lands. Option (a) keeps the change shippable; option (b) holds the Epic's order-4 slot hostage to a usage window nobody controls.

## Scope

- `src/store.rs` — product predicate semantics in `build_recall_query`.
- `tests/recall_filters_it.rs` — product-OR-unscoped coverage: product-tagged match, untagged-visible, wrong-product-excluded, global-namespace composition.
- Spec update: `docs/specs/0010-recall-metadata-filters.md` product-filter semantics (+ mirrored `.adv` copy, byte-identical).
- Probe protocol note recorded against concord#46.

## Out of scope

- Concord owning the Product entity.
- Episode importing Concord's data model or deriving Product membership from namespaces.
- Ingest-side product tagging (no source carries it).
- Option B typed column.

## Dependencies

- `addRecallMetadataFilters` — **shipped** (filter mechanism, spec 0010, `tests/recall_filters_it.rs`).
- Episode service restored — **resolved 2026-08-30**: binary rebuilt from `main` at `452ffb1`, migrations 0001-0003 applied, Vision entry re-enabled, MCP surface verified (`remember`, `recall`, `forget`, `promote`, `stats`), store at 0 rows.

## Language and storage assumption

Rust + Postgres + pgvector, per `decideStorageLanguageDirection`. This probe is the C20 reopen trigger: an insufficient outcome reopens C20 (episode ownership); only a reopened C20 resolving toward ownership brings the storage/language question back into scope. Nothing here fires it directly.

## Probe outcome flow

- Concord-side status: **C20 resolved 2026-08-30** as accepted decision **CD-0086** (concord PR #598; the earlier #596 was closed superseded by it): episode stays external, optional, and Product-scoped when configured. Automatic Product derivation and its real multi-project probe are recorded as episode-side follow-up work and the ownership reopen trigger.
- Outcome sufficient → CD-0086 stands; owning remains unjustified.
- Outcome insufficient → C20 reopens toward ownership; only then does storage/language return to scope.

## Acceptance criteria

- AC1: Recall scoped to a Product returns that Product's memories **and** global/untagged memories; wrong-product rows are excluded.
- AC2: Episode does not import or require Concord's data model (no Product-membership derivation).
- AC3 (Option B only — deferred): product column, backfill, partial index.
- AC4: The probe protocol and baseline are recorded against concord#46 so the eventual outcome is evidence-driven. *(Completion bar: see open question above.)*

## Epic context

Member of Epic `shapeEpisodeStructuredMemory` (advisory order 4 of 6). C20 reopen trigger. Named consumer: concord#46.