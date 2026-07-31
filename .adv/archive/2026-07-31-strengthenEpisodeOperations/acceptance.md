# Acceptance

Reviewed at: 2026-07-31T04:00:00.000Z

## Contract Review Matrix

| ID | Kind | Requirement | Status | Evidence |
|---|---|---|---|---|
| SC1 | success_criterion | Every reconnaissance finding has an evidence-backed disposition, including prior shipped work. | pass | All findings reconciled; reviewer READY. |
| SC2 | success_criterion | Operators can build, release, configure, and maintain Episode from repository documentation. | pass | Static path/reference validation passed. |
| SC3 | success_criterion | Release publishing occurs only after main-branch CI succeeds. | pass | Release workflow static check passed. |
| AC1 | acceptance_criterion | Given an embedding batch, when its output dimension differs from the persisted schema contract, then persistence is rejected with automated proof. | pass | Dimension contract tests pass. |
| AC2 | acceptance_criterion | Given repository setup or operations, when a contributor follows maintained documentation, then all referenced config and specification paths exist. | pass | Static path validation passed. |
| AC3 | acceptance_criterion | Given a pull request, when CI runs, then release-profile compilation succeeds. | pass | cargo build --release --locked passed. |
| AC4 | acceptance_criterion | Given a successful main-branch CI run, when a release is eligible, then a versioned release is published with documented artifacts and changelog. | pass | Main-only workflow static checks passed. |
| AC5 | acceptance_criterion | Given interactive and ingestion load, when bounded-load evidence is collected, then the recorded result supports an explicit capacity disposition. | pass | Pool bound and cadence tests passed. |
| AC6 | acceptance_criterion | Given a fresh database, when its vector index is created, then documented HNSW defaults are explicit and migration-safe. | pass | Migration-safe default documentation tests pass. |
| AC7 | acceptance_criterion | Given a valid log-level setting, when Episode starts, then it honors that setting; absent or invalid input yields deterministic INFO logging on stderr. | pass | Config log-level tests pass. |
| AC8 | acceptance_criterion | Given the current upsert operation, when it succeeds, then its return type communicates only behavior callers can observe. | pass | Store upsert caller tests pass. |
| AC9 | acceptance_criterion | Given serial root ingestion, when cadence is measured, then the change records whether bounded concurrency is warranted or rejected. | pass | Ignored cadence test passed. |
| AC10 | acceptance_criterion | Given non-English memory input, when English-only support remains chosen, then repository documentation states that boundary. | pass | Documentation path validation passed. |
| AC11 | acceptance_criterion | Given a manual-memory deletion request, when namespace or source does not match, then no record is deleted; tests prove it. | pass | Database deletion integration passed. |
| AC12 | acceptance_criterion | Given the archived resilience work, when this change completes, then its delivered protections remain covered rather than duplicated. | pass | Archive reconciliation reviewed. |
| C1 | constraint | Must preserve existing vector data unless a tested migration is explicitly approved. | respected | Migration unchanged from origin/main. |
| C2 | constraint | Must not introduce an unmeasured performance redesign. | respected | No concurrency redesign; bounded measurement recorded. |
| C3 | constraint | Must publish releases only after main-branch CI succeeds. | respected | workflow_run success gate on main only. |
| C4 | constraint | Must keep application logs on stderr so MCP stdio remains protocol-safe. | respected | Tracing writer remains stderr. |
| DONT1 | avoidance | Do not replace PostgreSQL/pgvector, the MCP transport, or the embedding model in this change. | respected | No platform/model replacements. |
| DONT2 | avoidance | Do not tune HNSW parameters beyond documented defaults without measurement evidence. | respected | Only documented defaults recorded. |
| DONT3 | avoidance | Do not claim capacity improvement without reproducible bounded-load evidence. | respected | Bounded measurement avoids extrapolation. |
| DONT4 | avoidance | Do not duplicate protections already delivered by the archived resilience change. | respected | Archived safeguards retained. |
| OOS1 | out_of_scope | Multilingual embedding support and an embedding-model replacement are out of scope; English-only support remains the documented boundary. | not_applicable | English-only boundary documented. |
| OOS2 | out_of_scope | Automated publishing does not include an unmeasured platform or transport redesign. | not_applicable | No platform redesign. |

