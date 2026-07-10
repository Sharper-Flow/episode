# Acceptance

Reviewed at: 2026-07-10T18:34:28.064Z

## Contract Review Matrix

| ID | Kind | Requirement | Status | Evidence |
|---|---|---|---|---|
| AC1 | acceptance_criterion | `forget` requires `namespace`; its persistence operation deletes only a matching `source = 'manual'` row in that namespace. Wrong namespace, ingested source, unknown id, and repeated deletion return `removed: 0`. | pass | Ignored Postgres deletion test passed; src/store.rs SQL predicate requires id, namespace, manual source. |
| AC2 | acceptance_criterion | Invalid or zero pool size/ingest interval, unknown embedding backend, and malformed/empty project-root entries cause startup validation to fail with the offending input name and expected format. | pass | Locked suite passes config validation tests in src/config.rs. |
| AC3 | acceptance_criterion | The SQLx pool config sets finite acquire, idle, and maximum-lifetime bounds. An unavailable database returns an acquisition error within the configured acquire limit. | pass | tests/pool_bounds.rs passed; explicit pool bounds in src/store.rs. |
| AC4 | acceptance_criterion | Ingestion uses a defined bounded batch size. With at least 65 eligible memories, it makes multiple batches rather than one unbounded call. | pass | Ignored ingest_batch_it 65-item partitions and persistence tests passed. |
| AC5 | acceptance_criterion | If recall queues during ingest, it is selected before the next ingestion batch begins. | pass | Scheduler recorder priority test passed in cargo test --locked. |
| AC6 | acceptance_criterion | Shutdown is cooperative: after active work returns, no new batch starts and the owned ingestion task exits before process completion. | pass | Scheduler and ingestion shutdown boundary tests passed. |
| AC7 | acceptance_criterion | Automated tests cover deletion namespace/source behavior, config validation, pool/store behavior, batching, recall fairness, and shutdown. Format, clippy, build, and tests pass. | pass | fmt, clippy, locked build, locked test, and ignored integration suites passed. |
| AC8 | acceptance_criterion | README and environment documentation accurately describe manual-only hard deletion, periodic ingestion, and unsupported Voyage behavior. CI includes dependency advisory checking; unused dependencies are removed and release profile settings are explicit. | pass | README/.env/CI/Cargo manifest inspected; cargo deny advisories passed. |
| C1 | constraint | Local MCP service only; no network-exposed multi-tenant authorization. | respected | Local stdio MCP architecture retained. |
| C2 | constraint | Retain Rust, pgvector, and BGE local embeddings. | respected | Rust, pgvector, and single local BGE embedder retained. |
| C3 | constraint | Use SQL predicates for deletion enforcement. | respected | Deletion enforcement is one SQL predicate in Store::forget_manual. |
| C4 | constraint | Keep one model instance; use bounded scheduling rather than a second BGE instance. | respected | Single embedder is moved into Scheduler worker; no second instance. |
| C5 | constraint | Use cooperative cancellation; do not rely on aborting active `spawn_blocking` inference. | respected | watch cancellation and awaited handles; no JoinHandle::abort. |
| DONT1 | avoidance | App-layer-only deletion authorization. | respected | Independent reviewer READY; no app-layer deletion authorization. |
| DONT2 | avoidance | Silent configuration fallbacks. | respected | Strict config parsing rejects invalid present values. |
| DONT3 | avoidance | Unbounded ingestion batches. | respected | INGEST_BATCH_MAX=64 and scheduler guards oversized jobs. |
| DONT4 | avoidance | New export, UI, or unrelated memory-product scope. | respected | Independent reviewer found no scope drift. |

