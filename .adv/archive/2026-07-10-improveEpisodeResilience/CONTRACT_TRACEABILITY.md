# Contract Traceability

**Change ID:** improveEpisodeResilience
**Contract Version:** 1
**Rigor:** standard
**Reviewed:** 2026-07-10T18:34:28.064Z

## Contract Items

| ID | Kind | Status | Evidence Policy | Evidence |
| --- | --- | --- | --- | --- |
| AC1 | acceptance_criterion | pass | test | Ignored Postgres deletion test passed; src/store.rs SQL predicate requires id, namespace, manual source. |
| AC2 | acceptance_criterion | pass | test | Locked suite passes config validation tests in src/config.rs. |
| AC3 | acceptance_criterion | pass | test | tests/pool_bounds.rs passed; explicit pool bounds in src/store.rs. |
| AC4 | acceptance_criterion | pass | test | Ignored ingest_batch_it 65-item partitions and persistence tests passed. |
| AC5 | acceptance_criterion | pass | test | Scheduler recorder priority test passed in cargo test --locked. |
| AC6 | acceptance_criterion | pass | test | Scheduler and ingestion shutdown boundary tests passed. |
| AC7 | acceptance_criterion | pass | test | fmt, clippy, locked build, locked test, and ignored integration suites passed. |
| AC8 | acceptance_criterion | pass | test | README/.env/CI/Cargo manifest inspected; cargo deny advisories passed. |
| C1 | constraint | respected | static_check | Local stdio MCP architecture retained. |
| C2 | constraint | respected | static_check | Rust, pgvector, and single local BGE embedder retained. |
| C3 | constraint | respected | static_check | Deletion enforcement is one SQL predicate in Store::forget_manual. |
| C4 | constraint | respected | static_check | Single embedder is moved into Scheduler worker; no second instance. |
| C5 | constraint | respected | static_check | watch cancellation and awaited handles; no JoinHandle::abort. |
| DONT1 | avoidance | respected | review | Independent reviewer READY; no app-layer deletion authorization. |
| DONT2 | avoidance | respected | review | Strict config parsing rejects invalid present values. |
| DONT3 | avoidance | respected | review | INGEST_BATCH_MAX=64 and scheduler guards oversized jobs. |
| DONT4 | avoidance | respected | review | Independent reviewer found no scope drift. |

## Task References

| Task | Implements | Verifies | Respects | N/A Reason |
| --- | --- | --- | --- | --- |
| tk-5724ed81c6dc | AC2, AC3 |  | C1, C2 |  |
| tk-252520f7698a | AC1 |  | C1, C3 | No separate verification obligation beyond AC1; task directly implements SQL predicate. |
| tk-b9e215a63d1a | AC4, AC5, AC6 |  | C2, C4, C5 | Implements all scheduler/lifecycle contract obligations; persistence batching follows in a dependent task. |
| tk-4dc6dd1eb05e | AC4, AC7 |  | C2, C4, DONT3 |  |
| tk-2732986d8239 | AC8 |  | C1, C2, DONT4 | Documentation, dependency manifest, release profile, and CI policy work; no new logic-bearing runtime path. |
| tk-896bf4f43adb |  | AC1, AC2, AC3, AC4, AC5, AC6, AC7, AC8 | C1, C2, C3, C4, C5, DONT1, DONT2, DONT3, DONT4 |  |
