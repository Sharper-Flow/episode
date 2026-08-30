# rq-episode-ingest-source-contract

## Statement

Ingestion is driven by source-agnostic parsers behind one trait; a new source is an implementation, not a refactor of reconcile.

## The trait

```rust
pub struct SourceParse {
    pub items: Vec<MemoryInput>,        // rows to store
    pub invalidated: Vec<String>,       // source ids withdrawn upstream
    pub promoted: Vec<String>,          // source ids graduated upstream
}

pub trait IngestSource {
    fn name(&self) -> &'static str;
    fn parse(&self, namespace: &str, project_root: &Path) -> Result<SourceParse>;
}
```

Implementations resolve their own paths and file formats from `project_root`; the trait is parser-shaped, not format-shaped. A source reading git-backed markdown with a manifest (Concord's CD-0026 lesson surface: working-tree files, publish-side git authority, idempotent publish, no retraction stream) fits without change. Parsing is synchronous and filesystem-bound.

## Ownership

Sources report; reconcile owns every store effect. The parse output feeds the existing fixed sequence — retraction before dedup, bulk source-id lookup, batched embed + upsert, promotion marking after the batch loop. No source writes to the store, holds a connection, or observes dedup state.

## Error isolation

One failing source warns (with its `name()`) and never blocks the others. A reconcile that cannot read one file must not be blinded to the remaining sources. Missing inputs yield an empty `SourceParse`, not an error.

## Store-level facts an implementer must know

- Retraction (`forget_ingested`) and promotion marking (`MARK_CANDIDATES_SQL`) restrict to `source IN ('adv_wisdom','adv_reflection')`. A new source is items-only by construction; giving it retraction or graduation semantics is a deliberate store change, not a parser change.
- Dedup (`existing_source_ids`) keys on `(namespace, source_id)` regardless of source. Every source must own a disjoint id space within a namespace, or its rows collide at the dedup boundary with another source's rows.
- Write-once semantics are unchanged (spec 0011): metadata freezes at first ingest, and post-ingest upstream changes flow only through `invalidated`/`promoted`.

## Current sources

`AdvWisdomSource` (`.adv/wisdom.jsonl`) and `AdvReflectionSource` (`.adv/reflections.jsonl`), aggregated by `ingest::aggregate` over `ingest::SOURCES` in a fixed order. Registering further sources is a code change to that list; no configuration surface exists until an external source needs one.

## Boundaries

This contract does not add ranking, recency scoring, hybrid retrieval, host capture, or Fleet behavior. The Concord lesson parser itself is future work — this contract is the surface it will implement.

## Verification

Unit tests prove aggregation order and error isolation with stub sources. The existing reconcile, promotion, and ingestion Postgres suites prove `AdvWisdomSource` and `AdvReflectionSource` preserve byte-for-byte store behavior through the trait (spec 0011's contract included).
