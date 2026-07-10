# Archive Briefing Digest

**Change ID:** improveEpisodeResilience
**Title:** Improve Episode resilience
**Status:** archived
**Generated:** 2026-07-10T19:24:41.372Z

## Identity Anchors

- CHANGE
- STATUS
- TERMINAL_GATE_SUMMARY
- Origin: discovery

## Archive Digest

**Status:** archived

| Gate | Status |
| --- | --- |
| proposal | done |
| discovery | done |
| design | done |
| planning | done |
| execution | done |
| acceptance | done |
| release | pending |

## Epic Context

No Epic membership

## Durable Facts

Showing 100 of 139 durable facts (39 omitted).

- **[agenda]** follow_ups: lib.rs `EmbedBackend::Voyage => anyhow::bail!(...)` arm is now unreachable via the env path (config rejects 'voyage' at validation); safe to collapse/simplify when lib.rs is in scope — not this task.
- **[agenda]** follow_ups: README documentation of the local-only backend and validation behavior is owned by AC8 task tk-2732986d8239 (out of scope here); only .env.example was updated in this task.
- **[archive_only_evidence]** decisions: Factored Config::from_env to delegate to a pure internal from_vars(iter) and unit-tested through it. — Rust 2024 marks std::env set_var/remove_var unsafe; a pure seam gives deterministic, parallel-safe tests with no env mutation and no unsafe blocks (P33).
- **[archive_only_evidence]** decisions: Pinned explicit pool constants (5s acquire, 10m idle, 30m max lifetime, test_before_acquire(true)) in a pub(crate) pool_options() builder instead of relying on sqlx defaults. — AC3 requires finite, explicit bounds; pinning them means a sqlx upgrade cannot silently change pool semantics, and the pure builder is unit-testable without a database.
- **[archive_only_evidence]** decisions: Proved the acquire bound with a never-accepting localhost TcpListener wrapped in a 20s tokio::time::timeout safety net. — The kernel completes the TCP handshake (backlog) but the PostgreSQL handshake never answers, so the client hangs inside connection establishment — exactly what acquire_timeout bounds — fully deterministic and DB-free. It drove the real eager connect+migrate path and returned an acquisition error in 5.00s.
- **[archive_only_evidence]** decisions: Left the EmbedBackend::Voyage variant and the lib.rs match arm in place; config rejects 'voyage' at validation. — lib.rs is outside this task's allowed file set (deletion/docs belong to other tasks); rejecting at validation satisfies AC2 without expanding scope. The now-unreachable arm is surfaced as a follow-up.
- **[archive_only_evidence]** decisions: Kept documented defaults for ABSENT optional vars while making PRESENT-but-invalid values error. — DONT2 forbids silent correction of invalid input, not documented defaults for unset vars; this preserves backward-compatible startup for the common case while eliminating the silent fallback.
- **[archive_only_evidence]** verification: CARGO_BUILD_JOBS=4 cargo test --lib config::tests (RED, before strict validation) (1) — RED: 10 strict-validation tests failed under the previous permissive parsing (zero/invalid pool, zero/invalid interval, unknown backend, voyage unsupported, malformed/empty/empty-ns/empty-path roots); 5 passed. Confirms AC2 behavior was missing.
- **[archive_only_evidence]** verification: CARGO_BUILD_JOBS=4 cargo test --test pool_bounds (RED, before pool bounds) (1) — RED: unavailable_db_errors_within_acquire_bound hung the full 20.00s safety timeout (Elapsed) under sqlx's 30s default acquire_timeout — acquire bound not enforced.
- **[archive_only_evidence]** verification: CARGO_BUILD_JOBS=4 cargo test (GREEN, after implementation) (0) — GREEN: 20 lib unit tests passed (15 config incl. all AC2 cases, 2 store pool-options pinning, 3 pre-existing ingest); tests/pool_bounds.rs passed in 5.00s (acquisition error within the 5s bound); recall_it remains ignored (needs DB/model); doctests 0.
- **[archive_only_evidence]** verification: cargo fmt --check (0) — Formatting clean across touched files.
- **[archive_only_evidence]** verification: CARGO_BUILD_JOBS=4 cargo clippy --tests -j 4 -- -D warnings (0) — Clippy clean with -D warnings (removed an unused test helper that tripped dead_code).
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo test --lib config::tests (RED, before strict validation)
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo test --test pool_bounds (RED, before pool bounds)
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo test (GREEN, after implementation)
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo fmt --check
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo clippy --tests -j 4 -- -D warnings
- **[archive_only_evidence]** decisions: Enforced deletion with a single static SQL statement carrying three predicates (`id`, `namespace`, `source = 'manual'`). — The predicate set is fixed (no dynamic columns), so a literal query is the simplest structural enforcement and satisfies C3/DONT1 without any pre-read or app-layer branch.
- **[archive_only_evidence]** decisions: Made `ForgetParams.namespace` a bare required `String` (no `Option`, no `#[serde(default)]`). — Makes `namespace` required at both the rmcp JSON-schema layer and serde deserialization, so 'forget requires namespace' is structural rather than a runtime guard.
- **[archive_only_evidence]** decisions: Marked the new deletion test `#[ignore = "requires Postgres"]` and seeded rows with a zero vector of `EMBEDDING_DIM` via `store.upsert`. — Keeps the default `cargo test` sweep DB-free (consistent with the existing integration test) while still running under `--ignored` against the dev DB; the zero vector avoids downloading the embedding model because the deletion predicate is model-independent.
- **[archive_only_evidence]** decisions: Used a separate raw-sqlx pool in the test for cleanup, scoped to the test's unique namespace. — Avoids adding a test-only method to the production `Store`; the unique namespace makes leftovers non-colliding even if cleanup fails.
- **[archive_only_evidence]** verification: cargo check --tests  # RED (pre-implementation): 5× E0599 no method named `forget_manual` found for struct `Store` — required API absent (101) — RED confirmed: forget_manual API/behavior absent before implementation (compile failed on the new test).
- **[archive_only_evidence]** verification: cargo test --test recall_it -- --ignored --nocapture forget_manual_enforces_namespace_and_manual_source (0) — GREEN: DB-backed deletion test passed — wrong namespace→0, ingested source→0, unknown id→0, matching manual→1, repeated delete→0, and the ingested row survived. (adv_run_test runId tr_mrf68f34_499d9412)
- **[archive_only_evidence]** verification: cargo fmt --check (0) — Format clean. (adv_run_test runId tr_mrf68o41_e4cc203f)
- **[archive_only_evidence]** verification: cargo clippy --all-targets -- -D warnings (0) — Clippy clean with warnings as errors. (adv_run_test runId tr_mrf68vhy_57db0443)
- **[archive_only_evidence]** verification: cargo build (0) — Binary/library build clean.
- **[archive_only_evidence]** verification: cargo test (0) — Default sweep green: 20 unit tests + tests/pool_bounds.rs pass; the two tests/recall_it.rs tests are #[ignore] (require Postgres).
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo check --tests  # RED (pre-implementation): 5× E0599 no method named `forget_manual` found for struct `Store` — required API absent
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test --test recall_it -- --ignored --nocapture forget_manual_enforces_namespace_and_manual_source
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo fmt --check
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo build
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test
- **[agenda]** follow_ups: Dependent task tk-252520f7698a: replace per-item exists_source/upsert in run_ingestion_loop with bulk source-id lookup + transactional bounded-batch upsert (static SQL/QueryBuilder). The scheduler already hands it bounded ≤64 batches.
- **[archive_only_evidence]** decisions: Owned the single embedder inside the scheduler worker; EpisodeServer and the ingestion loop now hold only a cloneable SchedulerHandle (bounded mpsc senders). — C4 requires one model instance and bounded scheduling rather than a second BGE model. Constructing LocalEmbedder once and moving it into the worker makes duplication structurally impossible in production.
- **[archive_only_evidence]** decisions: Biased tokio::select! branch order is shutdown -> interactive -> ingest, with a shutdown borrow check also at the top of each loop iteration. — When not shutting down, a queued interactive (recall/remember) job is always selected before another ingest batch (AC5). Listing shutdown first guarantees that once cancellation is signaled, no further batch is started even if ingest jobs are already queued (AC6).
- **[archive_only_evidence]** decisions: Worker awaits each spawn_blocking job to completion before selecting the next job (one active job at a time). — Preserves the one-model memory budget and makes fairness deterministic: an already-queued interactive job wins before the next ingest batch, while active spawn_blocking inference is never aborted (C5 — Tokio cannot cancel it).
- **[archive_only_evidence]** decisions: Empty and over-max (>64) batches are rejected at the SchedulerHandle before any job is scheduled; the ingestion loop additionally partitions with Vec::chunks(INGEST_BATCH_MAX). — AC4 requires a defined bounded batch size and that empty batches do not invoke embedding. Handle-level guards make the contract enforceable and directly testable regardless of caller.
- **[archive_only_evidence]** decisions: Added pure validate_batch_output (length + every-vector dimension) in the scheduler module and call it before per-item upsert; on mismatch the batch is skipped with structured namespace/batch context. — Design §3 batch contract: mismatches fail that batch and leave items eligible for the next reconcile. Placing the validator in the scheduler module keeps the batch contract with the scheduler and makes it unit-testable.
- **[archive_only_evidence]** decisions: Retained per-item exists_source dedup and per-item upsert in run_ingestion_loop instead of switching to bulk. — Bulk DB dedup/upsert is explicitly owned by the dependent task tk-252520f7698a; this task scopes to the scheduler + lifecycle only. Embedding is now bounded/batched; persistence batching is deferred.
- **[archive_only_evidence]** decisions: Barrier-based recorder tests use #[tokio::test(flavor = "multi_thread", worker_threads = 2)]. — They block the test thread on a std::sync::mpsc recv to observe the active embedder. On the default current_thread runtime that single-thread block deadlocked (the worker could not run); multi_thread lets the spawned worker progress on another thread and also better reflects the #[tokio::main] production runtime. Documented inline.
- **[archive_only_evidence]** verification: cargo build (0) — Finished dev profile; scheduler module + rewired server/lib compile clean.
- **[archive_only_evidence]** verification: cargo fmt -- --check (0) — No formatting diffs after cargo fmt.
- **[archive_only_evidence]** verification: cargo clippy --all-targets -- -D warnings (0) — Clippy clean with warnings as errors across all targets.
- **[archive_only_evidence]** verification: cargo test (0) — 29 lib tests + 1 pool_bounds integration test pass; 2 recall_it tests ignored (require Postgres + model download).
- **[archive_only_evidence]** verification: cargo test --lib scheduler:: (0) — 9 scheduler tests pass: shapes, empty-batch no-op, over-max rejection (AC4), recorder-proven interactive-before-next-ingest order (AC5), recorder-proven cooperative shutdown where active batch finishes and no next batch starts/worker exits (AC6), and batch-output length/dimension validation.
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo build
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo fmt -- --check
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test --lib scheduler::
- **[archive_only_evidence]** decisions: Wrapped each receiver in Option and introduced a Sel enum (Shutdown / InteractiveClosed / IngestClosed / Job); each select arm is guarded by `if <rx>.is_some()`. — A closed mpsc::Receiver returns None from recv() immediately and is therefore always ready. Leaving it in a biased select either spins or — as in the reported bug — maps to a loop exit that starves/abandons the other queue. The guard removes a closed channel from polling; the worker keeps servicing the open queue and exits only on shutdown or when both are closed. This is the structural fix the review asked for.
- **[archive_only_evidence]** decisions: Added two barrier/yield-driven regression tests (one per channel direction) that drive run_worker directly with separately controlled senders. — The bug only manifests when one sender class disappears while the other still has work. Dropping one sender in the test reproduces it deterministically: on the old code the surviving queue's job never starts (or its send fails), so the tests fail; on the fixed code they pass. Covering both directions pins the symmetric fix.
- **[archive_only_evidence]** decisions: Used the multi_thread flavor for the new tests (and kept it on the prior barrier tests). — They block the test thread on a std::sync::mpsc recv / rely on the worker making progress independently; the default current_thread runtime deadlocks, as established in attempt 1.
- **[archive_only_evidence]** verification: cargo test --lib scheduler:: (0) — 11 scheduler tests pass: original 9 plus 2 new regression tests (closed_interactive_does_not_terminate_ingest_queue, closed_ingest_does_not_terminate_interactive_queue).
- **[archive_only_evidence]** verification: cargo fmt -- --check (0) — No formatting diffs.
- **[archive_only_evidence]** verification: cargo clippy --all-targets -- -D warnings (0) — Clippy clean with warnings as errors.
- **[archive_only_evidence]** verification: cargo test --lib (0) — 31 lib tests pass (20 pre-existing + 11 scheduler).
- **[archive_only_evidence]** verification: cargo test (0) — Full sweep: 31 lib + 1 pool_bounds integration pass; 2 recall_it tests ignored (require Postgres + model).
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test --lib scheduler::
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo fmt -- --check
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test --lib
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test
- **[agenda]** follow_ups: The ignored integration tests in tests/ingest_batch_it.rs need a pgvector Postgres; they are not wired into any CI job. If CI gains a pgvector service, these can run with `--ignored` like recall_it.rs (separate task, not in scope here).
- **[archive_only_evidence]** decisions: Kept `Store::upsert` and `Store::exists_source` public and added new bulk methods alongside them, rather than deleting the single-row API. — `upsert` is still used by the `remember` MCP tool (server.rs) and `exists_source` by the `forget_manual` integration test; the task only requires removing them from the ingestion loop. Deleting them would widen scope into server.rs.
- **[archive_only_evidence]** decisions: Factored loop logic into pure helpers (`ingest_source_ids`, `filter_eligible`, `partition_ranges`) plus an async `reconcile_root`. — Makes AC4 batching (65 -> 2 partitions) and stable-id dedup unit-testable without a database or the infinite loop, while keeping scheduler/lifecycle behavior byte-for-byte intact (shutdown checks preserved in the loop wrapper and inside reconcile_root).
- **[archive_only_evidence]** decisions: `upsert_batch` takes parallel `&[MemoryInput]` + `&[Vec<f32>]` and re-asserts equal length. — Caller already proves length via `validate_batch_output`; the defensive re-check is cheap and makes misuse fail loudly. Avoids forcing the caller to allocate a zipped slice.
- **[archive_only_evidence]** decisions: Used a server-side `vector(1024)` wrong-dimension insert to force a deterministic transaction failure in the rollback test. — Production avoids this via `validate_batch_output`, but the store-level transaction must still be all-or-nothing; a wrong dim reliably aborts the multi-row INSERT and proves rollback without abstracting `Store` behind a trait (which would have touched server.rs).
- **[archive_only_evidence]** decisions: Namespace-prefixed all integration-test ids (`{ns}::{raw}`). — `memories.id` is a global (non-namespaced) primary key. Generic ids like `pw-1` collided across concurrently-running tests on the PK, producing a flaky 63-vs-65 count; qualifying ids makes tests concurrency-safe against the shared dev DB.
- **[archive_only_evidence]** verification: cargo fmt --check (0) — Formatting clean across src/ and tests/.
- **[archive_only_evidence]** verification: cargo clippy --all-targets -- -D warnings (0) — Clippy clean with warnings denied (lib + bins + integration tests).
- **[archive_only_evidence]** verification: cargo build (0) — Dev build succeeds.
- **[archive_only_evidence]** verification: cargo test (0) — Default sweep green: 36 lib unit tests (incl. 5 new: partition_65_yields_two_bounded_batches, partition_boundaries, ingest_source_ids_skips_manual_and_preserves_order, filter_eligible_dedups_and_preserves_stable_reflection_ids, filter_eligible_empty_existing_keeps_all) + pool_bounds; 5 Postgres-gated integration tests ignored.
- **[archive_only_evidence]** verification: cargo test --test ingest_batch_it -- --ignored (0) — Postgres-gated: bulk_dedup_returns_existing_subset_and_preserves_reflection_ids, batch_upsert_65_via_two_partitions_persists_all (64+1 -> 65 rows), failed_partition_rolls_back_and_does_not_poison_next (wrong-dim batch rolls back atomically; later partition persists) all pass; stable across repeated concurrent runs.
- **[archive_only_evidence]** verification: cargo test --test recall_it -- --ignored forget_manual (0) — Retained single-row `upsert` + `exists_source` still behave (AC1 deletion test passes) — no regression to the kept API.
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo fmt --check
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo build
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test --test ingest_batch_it -- --ignored
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo test --test recall_it -- --ignored forget_manual
- **[agenda]** follow_ups: Wire `tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env())` in src/main.rs so RUST_LOG is honored (make the documented knob real) — only if runtime log control is desired; currently documented as not-wired.
- **[agenda]** follow_ups: First CI run executes the live `cargo deny --locked check advisories` (needs network + RustSec DB); if a current vulnerability exists in the dep tree it will deny by design — triage rather than weakening the gate.
- **[agenda]** follow_ups: Stale module doc-comment in src/ingest.rs says the ingestion loop `lives in main.rs` (it lives in lib.rs) — outside AC8 docs scope; minor.
- **[agenda]** follow_ups: Optionally pin `EmbarkStudios/cargo-deny-action` to a release SHA (currently floating `@v2` major tag) for stricter supply-chain pinning.
- **[unresolved_action]** required_main_agent_actions: After merge/CI, confirm the new `Dependency advisories` step is green; if it denies a current vulnerability, triage the advisory (this is the gate functioning, not a wiring defect). I could not run cargo-deny locally (not installed; requires network + advisory DB).
- **[archive_only_evidence]** decisions: Removed direct deps `notify`, `chrono`, `thiserror` after a source scan (rg found 0 source references to any of them). — `notify` is a file-watcher for file-watch ingestion, which is unimplemented (ingestion is a periodic reconcile loop; README already said `currently periodic reconcile`) — crate + inotify subgraph fully removed. `chrono`/`thiserror` are never named in our code; both stay in the graph transitively (sqlx `chrono` feature / rmcp), so dropping the redundant direct edges is behavior-preserving. Proven safe by green fmt/clippy/build --locked/test and lock inspection.
- **[archive_only_evidence]** decisions: Added a measured `[profile.release]`: opt-level=3, lto="thin", codegen-units=1, strip="symbols"; left panic at the default "unwind" and overflow-checks at default. — Pins the perf/size knobs that matter for the single deployed `~/.local/bin/episode` binary while avoiding the much slower fat-LTO build and avoiding any change to unwinding semantics. Explicit/measured rather than maximal/aggressive.
- **[archive_only_evidence]** decisions: Added the advisory gate as `EmbarkStudios/cargo-deny-action@v2` with `command: check`, `command-arguments: advisories`, `arguments: --locked`, `rust-version: stable`, plus a repo-defined `deny.toml` scoped to advisories (vulnerability=deny; unmaintained/unsound/yanked/notice=warn). — Action input schema and `--locked` common option verified against upstream cargo-deny docs (common options go before the subcommand; `--locked` asserts Cargo.lock is unchanged; `-c` defaults to ./deny.toml). `rust-version: stable` is required because the crate uses edition 2024 (cargo >= 1.85). Scoped to advisories and notices-as-warn so a routine notice cannot retroactively break every build, while real vulnerabilities fail CI. Appended as a new step so fmt/clippy/build/test are unchanged.
- **[archive_only_evidence]** decisions: Corrected `.env.example` to state RUST_LOG is not consulted in v0 (commented the assignment) instead of wiring an EnvFilter in src/main.rs. — Verified in tracing-subscriber 0.3.23 source that `fmt().init()` uses a fixed default filter (INFO) and only honors RUST_LOG when `.with_env_filter(...)` is set, which main.rs does not. AC8 is a docs-alignment task (`align README/.env docs`) and explicitly says not to add features, so I aligned the doc to actual behavior and flagged EnvFilter wiring as an optional follow-up rather than changing runtime behavior.
- **[archive_only_evidence]** decisions: Fixed the stale `docker-compose.yml` comment from `Port 5433` to `Host port 5434`. — The actual mapping is `5434:5432`, matching `EPISODE_DATABASE_URL` and the README; the `5433` comment was factually wrong about supported configuration. Config-adjacent, factual, low-risk campsite fix within the docs-alignment scope.
- **[archive_only_evidence]** verification: cargo fmt --all --check (0) — Formatting clean (no diffs).
- **[archive_only_evidence]** verification: cargo clippy --all-targets -- -D warnings (0) — Clippy clean with warnings as errors after dep removal.
- **[archive_only_evidence]** verification: cargo build --all-targets --locked (0) — Build succeeds with the trimmed dependency set and the regenerated, locked Cargo.lock.
- **[archive_only_evidence]** verification: cargo test --locked (0) — 36 unit tests pass, 0 fail; Postgres/model integration tests are #[ignore] by design (3 ingest_batch_it, 2 recall_it); pool_bounds non-DB test passes; main + doctests 0.
- **[archive_only_evidence]** verification: cargo metadata --locked --no-deps --format-version 1 (0) — Cargo.toml and Cargo.lock are consistent (--locked accepted); root `episode` deps trimmed to the 12 remaining crates.
- **[archive_only_evidence]** verification: python3 tomllib load Cargo.toml + deny.toml (0) — Both parse; profile.release has opt-level/lto/codegen-units/strip; deny.advisories has db-path/db-urls/vulnerability(=deny)/unmaintained/unsound/yanked/notice; notify/chrono/thiserror confirmed absent from [dependencies].
- **[archive_only_evidence]** verification: yq parse .github/workflows/ci.yml (0) — YAML valid; existing Format/Clippy/Build/Test steps intact and contiguous; new `Dependency advisories` step uses EmbarkStudios/cargo-deny-action@v2 with rust-version=stable, command=check, command-arguments=advisories, arguments=--locked.
- **[unresolved_action]** consumer_warnings: verification_missing: No adv_run_test evidence found for reported command: cargo fmt --all --check

## Contract / AC Coverage

| ID | Kind | Status |
| --- | --- | --- |
| AC1 | acceptance_criterion | pass |
| AC2 | acceptance_criterion | pass |
| AC3 | acceptance_criterion | pass |
| AC4 | acceptance_criterion | pass |
| AC5 | acceptance_criterion | pass |
| AC6 | acceptance_criterion | pass |
| AC7 | acceptance_criterion | pass |
| AC8 | acceptance_criterion | pass |
| C1 | constraint | respected |
| C2 | constraint | respected |
| C3 | constraint | respected |
| C4 | constraint | respected |
| C5 | constraint | respected |
| DONT1 | avoidance | respected |
| DONT2 | avoidance | respected |
| DONT3 | avoidance | respected |
| DONT4 | avoidance | respected |

## Unresolved Actions

- verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo test --lib config::tests (RED, before strict validation)
- verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo test --test pool_bounds (RED, before pool bounds)
- verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo test (GREEN, after implementation)
- verification_missing: No adv_run_test evidence found for reported command: cargo fmt --check
- verification_missing: No adv_run_test evidence found for reported command: CARGO_BUILD_JOBS=4 cargo clippy --tests -j 4 -- -D warnings
- verification_missing: No adv_run_test evidence found for reported command: cargo check --tests  # RED (pre-implementation): 5× E0599 no method named `forget_manual` found for struct `Store` — required API absent
- verification_missing: No adv_run_test evidence found for reported command: cargo test --test recall_it -- --ignored --nocapture forget_manual_enforces_namespace_and_manual_source
- verification_missing: No adv_run_test evidence found for reported command: cargo fmt --check
- verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- verification_missing: No adv_run_test evidence found for reported command: cargo build
- verification_missing: No adv_run_test evidence found for reported command: cargo test
- verification_missing: No adv_run_test evidence found for reported command: cargo build
- verification_missing: No adv_run_test evidence found for reported command: cargo fmt -- --check
- verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- verification_missing: No adv_run_test evidence found for reported command: cargo test
- verification_missing: No adv_run_test evidence found for reported command: cargo test --lib scheduler::
- verification_missing: No adv_run_test evidence found for reported command: cargo test --lib scheduler::
- verification_missing: No adv_run_test evidence found for reported command: cargo fmt -- --check
- verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- verification_missing: No adv_run_test evidence found for reported command: cargo test --lib
- verification_missing: No adv_run_test evidence found for reported command: cargo test
- verification_missing: No adv_run_test evidence found for reported command: cargo fmt --check
- verification_missing: No adv_run_test evidence found for reported command: cargo clippy --all-targets -- -D warnings
- verification_missing: No adv_run_test evidence found for reported command: cargo build
- verification_missing: No adv_run_test evidence found for reported command: cargo test
- verification_missing: No adv_run_test evidence found for reported command: cargo test --test ingest_batch_it -- --ignored
- verification_missing: No adv_run_test evidence found for reported command: cargo test --test recall_it -- --ignored forget_manual
- After merge/CI, confirm the new `Dependency advisories` step is green; if it denies a current vulnerability, triage the advisory (this is the gate functioning, not a wiring defect). I could not run cargo-deny locally (not installed; requires network + advisory DB).
- verification_missing: No adv_run_test evidence found for reported command: cargo fmt --all --check
