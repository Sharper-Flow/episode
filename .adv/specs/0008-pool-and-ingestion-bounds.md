# rq-episode-pool-and-ingestion-bounds01

## Statement

Given interactive and ingestion load, when bounded-load evidence is collected, then the recorded result supports an explicit capacity disposition.

## Rationale

Episode's runtime protections already bound connection-pool acquisition, ingestion batch size, and active embedding to a single inference. Before considering any concurrency redesign, the change must collect reproducible, bounded evidence that those existing bounds actually operate as intended and that the current serial model is the throughput bottleneck.

## Authority

- `src/store.rs` pins `POOL_ACQUIRE_TIMEOUT`, `POOL_IDLE_TIMEOUT`, and `POOL_MAX_LIFETIME` and exposes them through `pool_options`.
- `src/scheduler.rs` enforces `INGEST_BATCH_MAX`, biased interactive-over-ingest selection, and one active inference at a time.
- `src/lib.rs` exposes `reconcile_root` as a testability seam for end-to-end cadence measurement.

## Verification

### Pool acquisition bound

- DB-free unit test `store::tests::pool_options_pin_lifecycle_bounds` asserts the explicit pool bounds are configured.
- Integration test `tests/pool_bounds.rs::unavailable_db_errors_within_acquire_bound` binds an unaccepting TCP listener and proves `Store::connect` returns a timeout error within the 5 s acquire bound (not the 20 s safety net).

Run:

```bash
cargo test --test pool_bounds
```

### Serial ingestion cadence

Integration test `tests/ingest_cadence.rs::serial_root_ingestion_cadence` drives the real reconcile path against the dev Postgres with a deterministic fake embedder. It records:

- workload: 64 unique wisdom items
- batch size: 64 (`INGEST_BATCH_MAX`)
- fake embedder delay: 10 ms/item
- pool config: `pool_size=10`, `acquire_timeout=5s`, `idle_timeout=10min`, `max_lifetime=30min`
- observed elapsed total: ~793 ms
- estimated serial model time: 640 ms
- pipeline overhead (elapsed − model): ~153 ms
- embedder invocations: 1
- ingested rows: 64

Run:

```bash
cargo test --test ingest_cadence -- --ignored --nocapture
```

### Interactive priority under load

- Scheduler unit test `scheduler::tests::interactive_job_selected_before_next_ingest_batch` proves a queued interactive job is selected before the next ingestion batch when both are ready.

Run:

```bash
cargo test --lib scheduler::tests::interactive_job_selected_before_next_ingest_batch
```

## Constraints

- Evidence must be reproducible on a local dev Postgres; do not extrapolate to production capacity.
- Do not introduce an unmeasured concurrency redesign (C2).
- Do not claim capacity improvement without bounded-load evidence (DONT3).

## Disposition

The local fastembed model is the sole throughput bottleneck: the fake-embedder measurement shows serial embedding time is ~85% of total reconcile time, and the remaining parse/dedup/DB overhead is small. Because the scheduler already pins exactly one active inference and prioritizes interactive jobs, bounded concurrency is **not warranted** in v0. No runtime concurrency redesign is proposed.
