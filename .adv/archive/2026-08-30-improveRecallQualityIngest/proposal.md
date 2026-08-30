## Cross-Project Origin

This change was created as a follow-up from **concord**.

| Field | Value |
|-------|-------|
| Source project | concord |
| Source path | `/home/jon/dev/concord` |

> **Note:** The originating project should be consulted for context on why this change is needed.


# Improve recall quality and ingest source abstraction

## Problem

Three quality/recency gaps remain after Tiers 1–2, plus one future-proofing gap. These are individually deferrable but worth scoping so they are not lost.

1. **No kind/source filtering at recall beyond the metadata filter.** Once Tier 1b lands, kind/source filtering falls out — but "only manual corrections, not ingested reflections" and "exclude success stories when looking for gotchas" are real recurring queries worth a first-class ergonomics path.
2. **No recency awareness.** A gotcha about a since-removed module ranks the same as a fresh one. There is no `max_age` filter and no score-blending.
3. **Ingest is hard-coupled to ADV.** `parse_wisdom`/`parse_reflections` read `.adv/wisdom.jsonl` + `reflections.jsonl` directly (ingest.rs). Concord now has its own learning-capture surface, so episode needs a source-agnostic ingest contract rather than deeper ADV coupling.

## Proposed change (three sub-parts, independently shippable)

### 3a. Kind/source filter ergonomics
- Promote `kind` and `source` to first-class recall filter params (alongside the metadata filter), so common queries are one field, not a metadata filter object.

> **Scope correction — half of 3a has shipped.** Tier 1b landed, and `RecallFilters` (`src/types.rs:134-140`) already carries `kinds: Option<Vec<String>>` as a first-class field, alongside `product`, `work_id`, `tags`, and `include_open_followups`. The `kind` half of 3a is delivered; building it again would duplicate shipped behavior.
>
> `source` is **not** present in `RecallFilters` and remains genuinely open. 3a should be re-scoped to `source` alone before any work starts, and AC1 narrowed accordingly. `source` also gains weight once 3c lands, since a second ingest source makes provenance filtering meaningful rather than decorative.

### 3b. Recency
- Add optional `max_age_days` to recall.
- Investigate score-blending (cosine x recency weight) vs pure filter. **Lean: pure filter first** — blending introduces tunable weights with no clear calibration, violating structural-correctness over heuristic. Filter is deterministic.

### 3c. Ingest source abstraction
- Introduce a source-agnostic ingest trait: a directory + a parser, not an ADV path.
- ADV becomes one parser implementation; Concord's lesson surface becomes another.
- Decouples episode from ADV's file layout without breaking current ingestion.

## Scope

- `src/ingest.rs`: trait + ADV parser impl (3c).
- `src/server.rs` / `src/store.rs`: `source` filter param (3a, re-scoped), max_age (3b).
- Tests per sub-part.

## Out of scope

- Auto-decay of memory quality scores (heuristic; defer).
- A Concord lesson parser implementation. The surface now exists (see below), but building the parser is separate work from establishing the trait that would host it.
- Re-implementing the `kind` filter, which has shipped.

## Concord's learning-capture surface now exists

An earlier draft deferred 3c partly on the grounds that Concord had no wisdom or reflection surface to abstract for. That is no longer true, and the correction matters because it converts 3c from speculative future-proofing into work with a second real consumer.

CD-0026 (Accepted) defines `concord_work_compact.lesson_publish`: a mutation on the compaction tool, gated by the `work_compact` capability and a separate operator approval. Publishing writes lesson markdown under `docs/lessons/`, appends its manifest record, and commits both through the repository's git authority in one commit. Reflections are not a separate stream — a reflection is a lesson carrying a `reflection` tag (CD-0026 D3), dispatched through `internal/agent/mutations.go`.

Two consequences for this change:

1. **The shape differs from ADV's.** ADV ingest reads JSONL streams; Concord's lessons are git-backed markdown plus manifest records. A trait that assumes "a directory of JSONL" would not host the second source. The abstraction must be parser-shaped, not format-shaped, which is what 3c already proposes — but the requirement is now concrete rather than anticipated.
2. **Episode has no parser for it today**, and this change does not add one. What changes is that 3c's motivation is now evidenced rather than hypothetical.

## Dependencies

- Depends on: `addRecallMetadataFilters` (3a builds on the filter mechanism). **This prerequisite has shipped** — `RecallParams.filters: Option<RecallFilters>` exists in `src/server.rs`, with spec `0010` and `tests/recall_filters_it.rs` on disk. The dependency is satisfied, and it partly satisfies 3a as noted above.
- 3c is independent of Tier 2.

## Language and storage assumption

This change is implemented in **Rust + Postgres + pgvector**, per the decision recorded in change `decideStorageLanguageDirection`.

That decision matters more here than for any other change in the epic, because this one depends directly on the capabilities it preserves. The decision considered and rejected Rust + SQLite — keeping the language while dropping the Postgres server — and one of the deciding grounds was retrieval and filtering:

- SQLite has no JSONB + GIN equivalent. sqlite-vec's `vec0` metadata columns are documented as slower on scans and inefficient past 12-character strings, which would directly weaken the recall filters spec `0010` already ships and that 3a extends.
- `vec0`'s stable KNN is exhaustive where pgvector offers HNSW.
- The candidate 3d hybrid-retrieval work below assumes Postgres-native `tsvector`/`tsquery` and `pg_trgm`. Under SQLite that becomes FTS5, with a different tokenizer trap, as the evidence section describes.

So the assumption is not incidental. If the storage decision is ever reopened at its option-decay checkpoint, this change's filter, recency, and hybrid-retrieval surface is the one that must be re-costed first.

## Acceptance criteria

- AC1 (3a): `recall` accepts `source` as a top-level filter param. *(Narrowed: `kinds` already ships in `RecallFilters`.)*
- AC2 (3b): `recall` accepts `max_age_days` and excludes older memories.
- AC3 (3b): No score-blending weights introduced without calibration evidence.
- AC4 (3c): Ingest is source-agnostic via a parser trait; ADV is one impl.
- AC5 (3c): Current ADV ingestion behavior is preserved by the refactored parser.
- AC6: Tests cover each sub-part.

## Epic context

Member of Epic `shapeEpisodeStructuredMemory` (advisory order 6 of 6). Polish and future-proofing tier.

## External evidence — magic-context

> Routed from change `studyMagicContextMemoryDesign`. Source: [`cortexkit/magic-context`](https://github.com/cortexkit/magic-context) (MIT), read at HEAD `a54f9c0`. This records a **candidate sub-part 3d**, not an adoption. AC1–AC6 are unchanged, and AC3's calibration bar governs this candidate.

### The gap this addresses

This change has no retrieval-mechanism sub-part. 3a and 3b filter results; neither changes how candidates are found and ranked in the first place.

magic-context pairs dense cosine with an FTS5 side table `memories_fts`, kept in sync by triggers. Scores are hybrid-combined, and FTS also serves as a standalone fallback when embeddings are unavailable.

Their stated motivation is a documented weakness of their embedding model: **weaker on exact symbols, file paths, and config keys than on prose.** `MEMORY-DESIGN.md` recommends benchmarking `bge-small-en-v1.5` if symbol and path lookups stay weak.

That weakness class matters here more than it does for them. episode's memories are ADV wisdom and reflection entries — text saturated with file paths, config keys, and symbol names. The weakness is inherent to dense retrieval, and episode's larger BGE-large model reduces it rather than removing it.

### A gap between their prose and their schema — this changes what actually transfers

magic-context's documentation describes its lexical index as tuned for technical tokens, preserving `.`, `_`, `/`, and `-`. Their actual DDL in `storage-db.ts` does not do that:

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
  content,
  category,
  content='memories',
  content_rowid='id',
  tokenize='porter unicode61'
);
```

`unicode61` is the FTS5 default tokenizer, and by default it treats every non-alphanumeric character as a separator. Preserving `.`, `_`, `/`, or `-` requires an explicit `tokenchars` argument, which appears nowhere in their schema. See the [SQLite FTS5 unicode61 documentation](https://www.sqlite.org/fts5.html#unicode61_tokenizer).

So `src/store.rs`-style path tokens are split on ingestion into their index, exactly as they would be by default. Two consequences:

1. **Their implementation is weaker evidence than their documentation.** Their hybrid retrieval is not tuned for technical tokens, so it does not demonstrate that the technique solves the symbol/path problem. The *idea* transfers; their *validation* of it does not.
2. **Postgres has the same trap.** Default `tsvector` parsers split path and symbol tokens similarly. A naive `tsvector` addition in episode would reproduce the identical gap while appearing to address it. This makes `pg_trgm` trigram matching the **load-bearing** half of any transfer here, not the optional half it first appears to be.

Transfer cost stays low — `tsvector`/`tsquery` and `pg_trgm` are native to Postgres, so no new dependency is required. But tokenizer configuration is the part that must be got right, and the source cannot be copied for it.

### Why this is a candidate and not a proposal

AC3 forbids introducing score-blending weights without calibration evidence. **A hybrid ranker is a blended score** and falls squarely under that bar.

Hybrid retrieval reduces the calibration problem — one mixing weight between lexical and dense scores, rather than the open-ended cosine-times-recency weighting 3b contemplates — but it does not eliminate it. Recording it as a bound candidate respects AC3. Adopting it here would violate AC3.

One calibration tool is worth naming if this is ever taken up: their `embedding-baseline.ts` and `embedding-baseline-diff.ts` snapshot top-K rankings per model and diff them by Kendall tau, with no gold labels. It is cheap and available to episode for the same question. It is also honest about its limit — it measures ranking *drift* between models, not ranking *correctness*, so it can tell you a change moved results without telling you it improved them.
