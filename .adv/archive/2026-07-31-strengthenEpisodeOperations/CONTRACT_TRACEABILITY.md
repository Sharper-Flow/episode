# Contract Traceability

**Change ID:** strengthenEpisodeOperations
**Contract Version:** 1
**Rigor:** standard
**Reviewed:** 2026-07-31T04:00:00.000Z

## Contract Items

| ID | Kind | Status | Evidence Policy | Evidence |
| --- | --- | --- | --- | --- |
| SC1 | success_criterion | pass | review | All findings reconciled; reviewer READY. |
| SC2 | success_criterion | pass | review | Static path/reference validation passed. |
| SC3 | success_criterion | pass | review | Release workflow static check passed. |
| AC1 | acceptance_criterion | pass | test | Dimension contract tests pass. |
| AC2 | acceptance_criterion | pass | test | Static path validation passed. |
| AC3 | acceptance_criterion | pass | test | cargo build --release --locked passed. |
| AC4 | acceptance_criterion | pass | test | Main-only workflow static checks passed. |
| AC5 | acceptance_criterion | pass | test | Pool bound and cadence tests passed. |
| AC6 | acceptance_criterion | pass | test | Migration-safe default documentation tests pass. |
| AC7 | acceptance_criterion | pass | test | Config log-level tests pass. |
| AC8 | acceptance_criterion | pass | test | Store upsert caller tests pass. |
| AC9 | acceptance_criterion | pass | test | Ignored cadence test passed. |
| AC10 | acceptance_criterion | pass | test | Documentation path validation passed. |
| AC11 | acceptance_criterion | pass | test | Database deletion integration passed. |
| AC12 | acceptance_criterion | pass | test | Archive reconciliation reviewed. |
| C1 | constraint | respected | static_check | Migration unchanged from origin/main. |
| C2 | constraint | respected | static_check | No concurrency redesign; bounded measurement recorded. |
| C3 | constraint | respected | static_check | workflow_run success gate on main only. |
| C4 | constraint | respected | static_check | Tracing writer remains stderr. |
| DONT1 | avoidance | respected | review | No platform/model replacements. |
| DONT2 | avoidance | respected | review | Only documented defaults recorded. |
| DONT3 | avoidance | respected | review | Bounded measurement avoids extrapolation. |
| DONT4 | avoidance | respected | review | Archived safeguards retained. |
| OOS1 | out_of_scope | not_applicable | not_applicable | English-only boundary documented. |
| OOS2 | out_of_scope | not_applicable | not_applicable | No platform redesign. |

## Task References

| Task | Implements | Verifies | Respects | N/A Reason |
| --- | --- | --- | --- | --- |
| tk-36dba6237a84 | AC1, AC6 | SC1 | C1, C2, DONT2 |  |
| tk-2b0ba1d52fdf | AC7, AC8, AC11 | SC1 | C4, DONT4 |  |
| tk-a14d74f7f95b | AC3, AC4 | SC3 | C3 |  |
| tk-fe925a7802b8 | AC2, AC10, AC12 | SC2 | DONT1, DONT4, OOS1 |  |
| tk-7084e982fe9e | AC5, AC9 | SC1 | C2, DONT3 |  |
