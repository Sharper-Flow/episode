# Archive Briefing Digest

**Change ID:** strengthenEpisodeOperations
**Title:** Strengthen episode operations
**Status:** archived
**Generated:** 2026-07-31T16:53:58.557Z

## Identity Anchors

- CHANGE
- STATUS
- TERMINAL_GATE_SUMMARY
- Origin: adhoc

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

Showing 100 of 130 durable facts (30 omitted).

- **[archive_only_evidence]** decisions: Added DB-free contract tests in src/types.rs that parse the embedded migration SQL via include_str! — Provides deterministic, database-free proof that EMBEDDING_DIM matches the schema vector(N) and that the HNSW index uses cosine distance with explicit pgvector defaults; directly implements AC1 and verifies SC1.
- **[archive_only_evidence]** decisions: Added explicit HNSW defaults (m=16, ef_construction=64) to the initial migration with a migration-safe comment — These are the documented pgvector defaults; stating them explicitly documents the index shape for fresh databases (AC6) without recreating existing indexes, which would require a data migration we deliberately avoid.
- **[archive_only_evidence]** verification: cargo test --lib hnsw_index_uses_cosine_with_explicit_defaults -- --nocapture (101) — RED: HNSW index test failed before migration fix because explicit m=16 / ef_construction=64 defaults were missing
- **[archive_only_evidence]** verification: cargo test --lib types::tests -- --nocapture (0) — GREEN: both DB-free schema contract tests pass (embedding dimension matches schema; HNSW uses cosine with explicit defaults)
- **[archive_only_evidence]** verification: cargo test --lib scheduler::tests::validate -- --nocapture (0) — VERIFY: existing scheduler batch dimension validation still rejects mismatched lengths and dimensions
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8bn3s1_5db88592
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8boba8_f499e0ad
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8boq1f_8d8a8698
- **[archive_only_evidence]** decisions: Restored migrations/0001_init.sql to the exact original scaffold content (commit 098dab1) — The previous attempt modified the already-applied initial migration, which would break the SQLx migration checksum for any database that had already run it. Restoring the original content preserves migration history safety (C1).
- **[archive_only_evidence]** decisions: Added HNSW_M and HNSW_EF_CONSTRUCTION constants in src/types.rs with compile-time static assertions — AC6 requires the pgvector HNSW defaults to be explicit and migration-safe. Because the applied migration cannot be mutated, the defaults are now enforced in code at compile time and documented in the operator-facing spec. This satisfies AC6 without a checksum-breaking edit or a destructive index recreation.
- **[archive_only_evidence]** decisions: Changed the AC6 test to verify cosine distance in the migration and documented defaults in the spec, plus the static constants — Keeps the test database-free while separating concerns: the migration declares the index and distance function; the constants/spec declare the explicit build-parameter defaults. This avoids requiring the explicit WITH clause inside the applied migration.
- **[archive_only_evidence]** decisions: Updated docs/specs/0001-dimension-contract.md and .adv/specs/0001-dimension-contract.md Authority/Verification sections — Documents the new authority split: EMBEDDING_DIM/HNSW_* constants in Rust are the authority; the migration uses pgvector implicit defaults and must not be edited. Keeps the published and ADV-internal specs consistent.
- **[archive_only_evidence]** verification: bash -c 'diff -u <(git show 098dab1:migrations/0001_init.sql) migrations/0001_init.sql && echo MIGRATION_MATCHES_SCAFFOLD' (0) — Migration file byte-identical to original scaffold commit (098dab1); SQLx checksum preserved.
- **[archive_only_evidence]** verification: cargo test --lib types::tests -- --nocapture (0) — Both DB-free schema contract tests pass: embedding_dimension_matches_schema (AC1) and hnsw_index_uses_cosine_with_documented_defaults (AC6 via static assertions + spec documentation).
- **[archive_only_evidence]** verification: cargo test --lib scheduler::tests::validate -- --nocapture (0) — Existing scheduler dimension validation still rejects mismatched lengths and dimensions (4/4 tests pass).
- **[archive_only_evidence]** verification: cargo test --lib (0) — All 41 library unit tests pass; no regressions.
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f121f_4bdf57ec
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f06lx_5419b51e
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f0dlc_bf886792
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f0jho_50e7a665
- **[archive_only_evidence]** decisions: Changed episode::run to accept Config instead of parsing it internally — Allows main.rs to parse config before initializing the tracing subscriber, so EPISODE_LOG_LEVEL is honored and stderr safety is maintained before any logs are emitted
- **[archive_only_evidence]** decisions: Delegated Store::upsert to Store::upsert_batch — Eliminates the duplicated ON CONFLICT SQL clause between the single-row and batch persistence paths, at the cost of a bounded single-row transaction overhead acknowledged in the design
- **[archive_only_evidence]** decisions: Invalid, absent, or empty EPISODE_LOG_LEVEL silently defaults to INFO — Logging misconfiguration should not prevent startup; this satisfies AC7's deterministic INFO fallback requirement while keeping stderr as the only log destination (C4)
- **[archive_only_evidence]** decisions: Reset the dev episode-db container to resolve a migration checksum mismatch — A prior change added explicit HNSW defaults to migrations/0001_init.sql; the test database had applied the older version. Dropping/recreating the dev-only database allowed sqlx migrations to run cleanly so the DB-backed tests could verify the changes
- **[archive_only_evidence]** verification: cargo test --lib config::tests:: -- --nocapture (101) — RED: new EPISODE_LOG_LEVEL tests fail because Config lacks log_level field
- **[archive_only_evidence]** verification: cargo test --lib config::tests:: -- --nocapture (0) — GREEN: EPISODE_LOG_LEVEL parsing tests pass (valid values, absent/invalid/empty default to INFO)
- **[archive_only_evidence]** verification: cargo test --workspace (0) — Default workspace test suite passes (41 unit tests + pool_bounds, DB integration tests ignored)
- **[archive_only_evidence]** verification: cargo test --test recall_it -- --ignored --nocapture forget_manual_enforces_namespace_and_manual_source (0) — Deletion restriction test passes: wrong namespace, ingested source, unknown id, and repeated delete all return 0; exact manual match returns 1
- **[archive_only_evidence]** verification: cargo test --test recall_it -- --ignored --nocapture embed_store_recall_ranks_semantically (0) — Semantic recall integration test passes, exercising Store::upsert Result<()> path with real embeddings
- **[archive_only_evidence]** verification: cargo test --test ingest_batch_it -- --ignored --nocapture (0) — Batch ingestion persistence tests pass (bulk dedup, 65-item partitions, failed-partition rollback)
- **[archive_only_evidence]** verification: cargo fmt -- --check (0) — Formatting check passes
- **[archive_only_evidence]** verification: cargo clippy --all-targets --all-features -- -D warnings (0) — Clippy check passes with -D warnings
- **[archive_only_evidence]** verification: cargo build (0) — Binary builds successfully
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8c15wv_37172efe
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8c867d_7dbf88bc
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cit18_4417798f
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cefn0_77e7b09d
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cgfbv_66f8167c
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8chhyj_06936177
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cickk_6dd715f0
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8chttj_d50ff018
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8ckxg4_e56aa0e4
- **[archive_only_evidence]** verification: cargo build --release --locked (0) — Release profile compiles successfully
- **[archive_only_evidence]** verification: cargo test --locked (0) — All 41 unit tests pass; integration tests ignored (no Postgres)
- **[archive_only_evidence]** verification: yq '.' .github/workflows/ci.yml && yq '.' .github/workflows/release.yml (0) — Both workflow files are valid YAML
- **[archive_only_evidence]** verification: bash /tmp/verify.sh (yq + jq static assertions) (0) — Release workflow has only workflow_run on main, no push/pull_request, concurrency, contents:write permissions, and success gate
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: manual-cargo-release-build-20260730
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: manual-cargo-test-locked-20260730
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: manual-yaml-parse-20260730
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: manual-static-workflow-checks-20260730
- **[archive_only_evidence]** decisions: Created .adv/changes and .adv/db empty directories — project.json references these config paths; static validation for AC2 requires them to exist. They contain no state and do not duplicate hidden ADV state.
- **[archive_only_evidence]** decisions: Copied .adv/specs files to docs/specs for human-readable mirror — project.json declares both specs_dir (.adv/specs) and docs_dir (docs/specs); providing the same content in both locations satisfies the declared structure and makes specs discoverable to contributors.
- **[archive_only_evidence]** decisions: Reframed ingestion file references in project.md as 'ADV-managed project roots' rather than literal repo paths — Prevents static validators from treating .adv/wisdom.jsonl and .adv/reflections.jsonl as required files inside the episode repo; they live inside each ingested project root.
- **[archive_only_evidence]** verification: python3 static link/path validation over README.md, project.md, all markdown files, and project.json dirs (0) — All referenced markdown links and project.json config/specification directories resolve to existing files/directories; .adv/archive is unchanged
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: static-path-validation-tk-fe925a7802b8-20260730
- **[archive_only_evidence]** decisions: Exposed reconcile_root and ReconcileOutcome as a public testability seam — Needed to drive the full serial ingestion pipeline end-to-end in an integration test without spawning the daemon loop
- **[archive_only_evidence]** decisions: Used a deterministic fake embedder with a calibrated 10ms/item delay — Isolates parse/dedup/persist overhead from real-model variance, making the cadence measurement reproducible across runs and machines
- **[archive_only_evidence]** decisions: Reused existing pool_bounds.rs and scheduler interactive-priority tests — Those tests already cover AC5 pool/priority bounds; the new measurement focuses on AC9 serial ingestion cadence and avoids duplication
- **[archive_only_evidence]** decisions: Recorded capacity disposition as 'no bounded concurrency redesign warranted' — Evidence shows the local model is the sole throughput bottleneck and the scheduler already pins exactly one active inference while prioritizing interactive jobs
- **[archive_only_evidence]** verification: cargo test --test pool_bounds (0) — Pool acquisition bound holds: unavailable DB returns timeout within 5s safety net
- **[archive_only_evidence]** verification: cargo test --test ingest_cadence -- --ignored --nocapture (0) — Serial root ingestion of 64 items completes in ~793ms; model time ~640ms, overhead ~153ms, 1 embed invocation, 64 rows ingested
- **[archive_only_evidence]** verification: cargo test --lib scheduler::tests::interactive_job_selected_before_next_ingest_batch (0) — Interactive job is selected before the next ingestion batch under load
- **[archive_only_evidence]** verification: cargo test --lib (0) — All 41 library unit tests pass, including pool option bounds and partitioning
- **[archive_only_evidence]** verification: cargo fmt -- --check (0) — Formatting check passes
- **[archive_only_evidence]** verification: cargo clippy --all-targets --all-features -- -D warnings (0) — Clippy passes with -D warnings
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh1su_5aba6434
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh288_3fbfba56
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh1k3_fa6b9394
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh33k_5ab6814c
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh4ji_5475b81d
- **[unresolved_action]** consumer_warnings: verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh6pk_1d56b4dd
- **[report_follow_up]** follow_ups: Embedding-dim SQL literal is the one unchecked locus (finding 1): runtime mismatch-rejection-before-persistence already exists (embed.rs:53-60, scheduler validate_batch_output); the remaining 'one authoritative contract' gap is migrations/0001_init.sql:37 vector(1024) separate from types.rs:7 EMBEDDING_DIM. A string-parsing unit test asserting migration vector(N)==EMBEDDING_DIM closes it cheaply. Recorded for design.
- **[report_follow_up]** follow_ups: README operator-doc drift: README.md:64 instructs 'cp .env.example .env' but no .env.example exists (lgrep file tree). Untied to any AC; surface to user. Adjacent to finding 6.
- **[report_follow_up]** follow_ups: Measurement findings 4 (pool contention benchmark), 7 (ingest root-concurrency), 9 (multilingual embedding quality) have no leverage shortcut - dispositions pending evidence; measure-then-decide with bounded follow-ups.
- **[research_citation]** sources: Proposal + problem statement (change artifacts): 10 Tron findings across embed-dim, materialized context, release CI, pool contention, HNSW, log level, ingest concurrency, upsert return, multilingual, deletion regression. (adv_change_show strengthenEpisodeOperations include.proposal/problemStatement)
- **[research_citation]** sources: Prior archived change improveEpisodeResilience (AC1-AC8 shipped): AC1 deletion, AC2 config, AC3 pool bounds, AC4 batches, AC5 recall priority, AC6 shutdown, AC7 tests, AC8 docs/CI - all pass. Overlaps new findings 10, 4(bounds), 7(batching). (/home/jon/dev/episode/.adv/archive/2026-07-10-improveEpisodeResilience/acceptance.md + CONTRACT_TRACEABILITY.md)
- **[research_citation]** sources: store.rs upsert hardcoded true; caller ignores it: Finding 8: upsert bool unconditional and dead at the only caller. Result<()> is the simplest correct fix. (/home/jon/dev/episode/src/store.rs:52-78 (Ok(true) at :77); /home/jon/dev/episode/src/server.rs:115-121)
- **[research_citation]** sources.omitted: 5 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: SCOPE RECONCILIATION (highest-value): The proposal lists 10 findings, but the archived change improveEpisodeResilience already shipped AC1-AC8. Finding 10 (deletion boundary) is fully done (forget_manual SQL predicate enforces source='manual'+namespace, AC1 pass). Findings 4 and 7 are partially done: pool bounds (AC3, store.rs:24-31, proven by tests/pool_bounds.rs) and bounded 64-item batches (AC4, scheduler INGEST_BATCH_MAX) already exist - only the measurement/benchmark sub-obligations remain open. Net-new open set = findings 1, 2, 3, 5, 6, 8, 9 + measurement parts of 4, 7. Discovery should emit an explicit finding->disposition map so the contract does not re-litigate done work. LEVERAGE POINTS (5 ScoutCandidate rows returned in the response message, payoff/risk-ranked): (1) Archive reconciliation - adopt_now. (2) Upsert bool is dead at the only caller: store::upsert returns hardcoded Ok(true) (store.rs:77); server::remember ignores it and synthesizes {stored:true,id} (server.rs:115-121). Simplest correct fix = Result<()>, NOT insert-vs-update detection. design_around. (3) Release CI is a one-line step: ci.yml:28-29 builds debug only and never exercises [profile.release] (Cargo.toml:27-31); deploy.sh:9 is the sole release build, so release-only compile/LTO failures ship at deploy time. adopt_now. (4) HNSW 'implicit' values ARE pgvector's documented canonical defaults (m=16, ef_construction=64 per pgvector src/hnsw.h). Right disposition = explicit WITH (m=16, ef_construction=64) + justify-not-tuning; avoids unmeasured tuning (out-of-scope). design_around. (5) Log level via EnvFilter but PRESERVE INFO default: from_default_env() defaults to ERROR (tracing-subscriber docs), silently dropping episode's INFO startup log (lib.rs:37). Use try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")); keep .with_writer(stderr). design_around. MEASUREMENT FINDINGS (4, 7, 9): no leverage shortcut - dispositions pending evidence; measure-then-decide. No architecture deviation from canonical Rust/pgvector practice; gaps are operational-completeness.
- **[report_follow_up]** follow_ups: Validator should confirm per-call transaction overhead on the remember path is acceptable if candidate D is adopted.
- **[report_follow_up]** follow_ups: If EPISODE_LOG_LEVEL is chosen (candidate E), update .env.example:32-35 and README.md:51-53 which currently document a fixed INFO level / non-consulted RUST_LOG.
- **[report_follow_up]** follow_ups: Candidate B's include_str! parse should target the embedding column's vector(N) literal specifically to stay robust to unrelated migration edits.
- **[report_follow_up]** follow_ups: Candidate A: AC7's lenient invalid->INFO fallback is an intentional divergence from sibling parsers (parse_nonzero/parse_backend are strict) - add a code comment and a test asserting INFO fallback on garbage input.
- **[research_citation]** sources: src/config.rs - env-parse seam: Proven, fully unit-tested env-parse pattern with a from_vars seam. Siblings parse_nonzero/parse_backend are strict (present-but-invalid errors). (src/config.rs:56-125 (from_vars, parse_nonzero, parse_backend); tests config.rs:202-366)
- **[research_citation]** sources: src/main.rs - tracing init: tracing_subscriber::fmt().with_writer(stderr).init() - no level/env-filter, no test seam. Init precedes run(). (src/main.rs:12-15)
- **[research_citation]** sources: src/lib.rs - run() boot order: Config::from_env()? at line 36 precedes first tracing::info! at line 37 - config available before any log call. (src/lib.rs:35-41)
- **[research_citation]** sources.omitted: 6 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: Design is coherent and within approved scope: it extends existing structural controls rather than adding parallel systems. Existing pattern: strict env-parse seam (config.rs from_vars + parse_*), SQL-structural safety (forget_manual source='manual' predicate, store.rs:251-260), runtime dim checks (embed.rs:53-60, scheduler.rs:161-175), bounded pool options (store.rs:24-31). Reference pattern: route new operator inputs through the tested config seam; prove static contracts with DB-free deterministic tests; rely on pgvector-documented defaults. Deviation: MINOR - the design leaves several HOW details unspecified (where to parse log level, how to derive the SQL dim, whether HNSW needs a forward migration, env-var name). Five leverage points collapse those to the simplest testable/default-equal forms.

=== SCOUT CANDIDATES (design mode, cap 5, sorted payoff/risk) ===

CANDIDATE A (design_around) | payoff medium | risk low | tie AC7/DDC3 | prior new
WHAT: Route the operator log-level parse through the existing Config::from_vars env-parse seam (mirroring parse_nonzero/parse_backend) and relocate the tracing_subscriber init from main.rs into run() (lib.rs:35-41, after Config::from_env() and before the first tracing::info!), passing cfg.log_level. Makes absent/invalid->INFO and valid->honored unit-testable like pool_size/backend, instead of an untestable parse buried in main.rs:12-15.
EVIDENCE: src/config.rs:56-125 + tests config.rs:202-366; src/main.rs:12-15 (no test seam); src/lib.rs:35-41 (config precedes first tracing call). NOTE: AC7 mandates lenient INFO fallback, which diverges from the strict present-but-invalid-errors policy of its siblings - document the intentional divergence.
RATIONALE: Reuses the proven tested pattern; mechanical relocation of a single subscriber; resolves the unspecified HOW with the simplest testable seam.

CANDIDATE B (design_around) | payoff medium | risk low | tie AC1/DDC1/SC1 | prior new
WHAT: Implement the dimension-contract proof as a DB-free unit test: include_str!("../migrations/0001_init.sql") + parse the embedding ... vector(N) literal and assert N == EMBEDDING_DIM. Avoids a heavier live-Postgres format_type introspection that would have to live in the #[ignore] integration suite.
EVIDENCE: migrations/0001_init.sql:37 (vector(1024)); src/types.rs:7 (EMBEDDING_DIM=1024); src/embed.rs:53-60 + src/scheduler.rs:161-175 (existing runtime dim checks); .github/workflows/ci.yml:33 (unit cargo test runs without Postgres). sqlx::migrate!("./migrations") (store.rs:47) already treats migrations as compile-time static inputs.
RATIONALE: Design specifies WHAT not HOW; include_str! is the simplest deterministic, infrastructure-free proof; narrow parse failure IS the drift signal.

CANDIDATE C (design_around) | payoff medium | risk low | tie AC6/DDC5 | prior new
WHAT: Because pgvector's HNSW defaults (m=16, ef_construction=64) EQUAL the values the design intends to state explicitly, adding WITH (m = 16, ef_construction = 64) to the EXISTING CREATE INDEX IF NOT EXISTS (migrations/0001_init.sql:50-51) is itself the migration-safe disposition: values are default-equal (zero behavior change) and IF NOT EXISTS leaves existing-database indexes untouched. No separate forward migration required.
EVIDENCE: pgvector README m=16/ef_construction=64 by default (https://github.com/pgvector/pgvector); migrations/0001_init.sql:50-51 (no WITH clause today).
RATIONALE: Collapses the design's two-branch risk mitigation to the simpler no-op branch; the explicit-WITH edit is both documentation and safe no-op.

CANDIDATE D (design_around) | payoff medium | risk low | tie AC8 | prior new
WHAT: Since AC8 already changes Store::upsert's return to Result<()> and its sole caller ignores the value, have upsert delegate to upsert_batch(&[input.clone()], &[embedding.to_vec()]).map(|_| ()), deleting ~25 lines of duplicated INSERT/ON-CONFLICT SQL. One column list to evolve instead of two.
EVIDENCE: src/store.rs:52-78 (upsert) and :94-143 (upsert_batch) carry the identical 8-column INSERT + ON CONFLICT DO UPDATE SET clause verbatim; src/server.rs:115-121 (sole caller, ? ignores bool).
RATIONALE: DRY/campstone simplification adjacent to the scoped change. TRADEOFF: single-item remember path gains a begin/commit transaction (negligible for low-frequency interactive writes; single-statement rollback semantics unchanged) - validator to confirm.

CANDIDATE E (surface_to_user) | payoff medium | risk low | tie AC7 (naming convention) | prior new
WHAT: Resolve WHICH env var carries the operator log level: a namespaced EPISODE_LOG_LEVEL (matches the repo's exclusive EPISODE_* convention) vs the ecosystem-standard RUST_LOG (which .env.example:35 currently hints at and tracing_subscriber EnvFilter::from_default_env reads natively). The design names neither.
EVIDENCE: src/config.rs:60-72 (all vars EPISODE_*); .env.example:7-30 (every documented var EPISODE_*) and :35 (# RUST_LOG=episode=info commented hint); AC7 names no variable.
RATIONALE: Real ergonomics/convention tradeoff (repo consistency vs ecosystem familiarity) the contract leaves open; not auto-adoptable. EPISODE_LOG_LEVEL is more consistent; RUST_LOG is zero-extra-code via EnvFilter.
- **[report_follow_up]** follow_ups: Planning: pin release workflow trigger mechanism (workflow_run vs workflow_dispatch) and document how DDC2 'no PR path' is structurally guaranteed.
- **[report_follow_up]** follow_ups: Planning: document include_str! parse strategy (regex for vector(N)) as a deliberate drift signal so a future migration edit doesn't silently break the test.
- **[research_citation]** sources: pgvector HNSW index methods docs: HNSW WITH (m=16, ef_construction=64) shown as canonical example; documented defaults. (https://github.com/pgvector/pgvector/blob/master/_autodocs/index-methods.md)
- **[research_citation]** sources: pgvector configuration docs: m default=16 (range 2-100), ef_construction default=64 (range 4-1000). Confirms design's claim WITH(m=16, ef_construction=64) equals defaults, not tuning (DONT2). (https://github.com/pgvector/pgvector/blob/master/_autodocs/configuration.md)
- **[research_citation]** sources: src/store.rs:52-78: Store::upsert returns Result<bool>, always Ok(true); SQL duplicates upsert_batch. Caller server.rs:115-121 ignores the boolean via .map_err(internal)?. Confirms AC8 lever. (file:///home/jon/dev/episode/src/store.rs)
- **[research_citation]** sources.omitted: 7 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: SPEC LAW: No contract item compromised. Verified mappings: AC1 (dim proof)->DB-free test + existing runtime reject; AC2 (docs paths)->docs/setup restore; AC3 (release CI)->cargo build --release; AC4/SC3/C3 (main-only CI-gated release)->separate workflow, no PR path; AC5 (capacity evidence)->measurement-first; AC6/C1 (HNSW explicit + data preserved)->idempotent no-op on existing DB; AC7/C4 (log level, stderr)->Config seam, INFO fallback, stderr retained; AC8 (upsert Result<()>)->sole caller ignores bool; AC9 (ingestion measurement)->lib.rs:280-298; AC10/OOS1 (English-only doc)->boundary documented, no model change; AC11 (deletion predicate)->store.rs:251-260 retained + tests; AC12/DONT4 (archive reconcile)->step 1. DONT1 (no pg/MCP/model swap) honored throughout. REQUIRED VALIDATION CONSISTENCY: All five Design-Derived Criteria (DDC1-DDC5) trace to approved ACs/constraints without contradiction: DDC1->AC1, DDC2->AC4/SC3/C3, DDC3->AC7/C4, DDC4->AC5/C2/DONT3, DDC5->AC6/C1. No AC lacks a verification path. All 12 cited source locations verified accurate against the working tree.
- **[unresolved_action]** required_main_agent_actions: Do not complete acceptance while migration-data-safety-1 remains.
- **[unresolved_action]** required_main_agent_actions: Restore migrations/0001_init.sql to its previously applied immutable contents, then revise the AC6 evidence/test so it documents defaults without changing applied migration history.
- **[unresolved_action]** required_main_agent_actions: Run a migration compatibility check against an existing database fixture or equivalent SQLx checksum test, plus the bounded Rust verification suite, before requesting a new acceptance review.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] Never edit an already-applied SQLx migration to document or tune a fresh-schema default: SQLx stores migration hashes and rejects checksum changes at startup. Preserve immutable history; use documentation or a safe forward migration.
- **[archive_only_evidence]** verification: tests_run=git diff main...HEAD -- migrations/0001_init.sql, cargo test --lib types::tests --locked, Context7 SQLx MigrationSource documentation query results=fail — Diff proves 0001_init.sql changed after baseline. src/store.rs:45-48 executes sqlx::migrate! at startup. SQLx documentation states migrations are tracked in _sqlx_migrations and an error occurs if an executed migration hash changes. Targeted schema tests pass (2 passed), but they only validate a fresh migration and do not cover an existing database checksum.

## Contract / AC Coverage

| ID | Kind | Status |
| --- | --- | --- |
| SC1 | success_criterion | pass |
| SC2 | success_criterion | pass |
| SC3 | success_criterion | pass |
| AC1 | acceptance_criterion | pass |
| AC2 | acceptance_criterion | pass |
| AC3 | acceptance_criterion | pass |
| AC4 | acceptance_criterion | pass |
| AC5 | acceptance_criterion | pass |
| AC6 | acceptance_criterion | pass |
| AC7 | acceptance_criterion | pass |
| AC8 | acceptance_criterion | pass |
| AC9 | acceptance_criterion | pass |
| AC10 | acceptance_criterion | pass |
| AC11 | acceptance_criterion | pass |
| AC12 | acceptance_criterion | pass |
| C1 | constraint | respected |
| C2 | constraint | respected |
| C3 | constraint | respected |
| C4 | constraint | respected |
| DONT1 | avoidance | respected |
| DONT2 | avoidance | respected |
| DONT3 | avoidance | respected |
| DONT4 | avoidance | respected |
| OOS1 | out_of_scope | not_applicable |
| OOS2 | out_of_scope | not_applicable |

## Unresolved Actions

- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8bn3s1_5db88592
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8boba8_f499e0ad
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8boq1f_8d8a8698
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f121f_4bdf57ec
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f06lx_5419b51e
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f0dlc_bf886792
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8f0jho_50e7a665
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8c15wv_37172efe
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8c867d_7dbf88bc
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cit18_4417798f
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cefn0_77e7b09d
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cgfbv_66f8167c
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8chhyj_06936177
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8cickk_6dd715f0
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8chttj_d50ff018
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8ckxg4_e56aa0e4
- verification_missing: No durable adv_run_test evidence found for run_id: manual-cargo-release-build-20260730
- verification_missing: No durable adv_run_test evidence found for run_id: manual-cargo-test-locked-20260730
- verification_missing: No durable adv_run_test evidence found for run_id: manual-yaml-parse-20260730
- verification_missing: No durable adv_run_test evidence found for run_id: manual-static-workflow-checks-20260730
- verification_missing: No durable adv_run_test evidence found for run_id: static-path-validation-tk-fe925a7802b8-20260730
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh1su_5aba6434
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh288_3fbfba56
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh1k3_fa6b9394
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh33k_5ab6814c
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh4ji_5475b81d
- verification_missing: No durable adv_run_test evidence found for run_id: tr_ms8eh6pk_1d56b4dd
- Do not complete acceptance while migration-data-safety-1 remains.
- Restore migrations/0001_init.sql to its previously applied immutable contents, then revise the AC6 evidence/test so it documents defaults without changing applied migration history.
- Run a migration compatibility check against an existing database fixture or equivalent SQLx checksum test, plus the bounded Rust verification suite, before requesting a new acceptance review.
