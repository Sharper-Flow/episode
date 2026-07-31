# rq-episode-dimension-contract01

## Statement

Given model output and the persisted schema configuration, when an embedding batch is accepted, then the accepted dimension matches the authoritative Rust contract before persistence.

## Rationale

A mismatch between the embedding model output dimension, the Rust `EMBEDDING_DIM` constant, and the SQL schema `vector(N)` column would corrupt stored vectors or cause runtime failures. The contract must be checkable without a live database.

## Authority

- `EMBEDDING_DIM` in `src/types.rs` is the Rust authority for vector dimension.
- `HNSW_M` and `HNSW_EF_CONSTRUCTION` in `src/types.rs` are the Rust authority for the pgvector HNSW build defaults (`m = 16`, `ef_construction = 64`).
- `migrations/0001_init.sql` declares the schema `vector(N)` and the HNSW index using pgvector's implicit defaults; the explicit default values are documented here and enforced by static assertions in `src/types.rs`.

## Verification

- A DB-free unit test parses the migration via `include_str!` and asserts that the schema dimension equals `EMBEDDING_DIM`.
- A DB-free unit test asserts the HNSW index uses cosine distance and that this spec documents the explicit pgvector defaults (`m = 16`, `ef_construction = 64`); static assertions on `HNSW_M` and `HNSW_EF_CONSTRUCTION` enforce the values at compile time.
- Runtime batch validation in `src/scheduler.rs` rejects dimensions that do not equal `EMBEDDING_DIM` before calling `Store::upsert_batch`.

## Constraints

- Existing vector data must be preserved; recreating indexes or columns requires an explicit migration.
- HNSW parameters must remain pgvector documented defaults (`m = 16`, `ef_construction = 64`) unless measurement evidence justifies a change.
