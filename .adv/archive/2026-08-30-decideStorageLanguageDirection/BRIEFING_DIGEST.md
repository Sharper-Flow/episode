# Archive Briefing Digest

**Change ID:** decideStorageLanguageDirection
**Title:** Decide storage and language direction
**Status:** archived
**Generated:** 2026-08-30T15:19:14.337Z

## Identity Anchors

- CHANGE
- STATUS
- TERMINAL_GATE_SUMMARY

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

Epic: shapeEpisodeStructuredMemory · Storage/language direction decision (order 1)

## Durable Facts

Showing 18 of 18 durable facts.

- **[report_follow_up]** follow_ups: Edit the decision record (proposal.md): correct the C1 corollary, add the C2 feature-gate note, add Rust + SQLite to alternatives, rewrite the trigger/probe-status section with the three blockages, the predecessor-retirement deadline, and the operator-approved concord#46 C20 resolution; add an option-decay checkpoint.
- **[report_follow_up]** follow_ups: Operational (episode-side, outside this epic): re-enable episode on the host and record the disablement cause; without a recorded cause the deployment-friction question stays open.
- **[report_follow_up]** follow_ups: Sequencing risk tracked by concord#101/#46: addProductScopingRecall does not exist and the ingestion source retires with the predecessor — implement or re-point before the window closes, or explicitly retire the trigger.
- **[research_citation]** sources: rusqlite docs (docs.rs): load_extension_enable/disable, LoadExtensionGuard, load_extension; requires load_extension crate feature (https://docs.rs/rusqlite/latest/rusqlite/struct.LoadExtensionGuard.html)
- **[research_citation]** sources: mattn/go-sqlite3 wiki - Features: External extension loading enabled by default; sqlite_omit_load_extension disables (https://github.com/mattn/go-sqlite3/wiki/Features)
- **[research_citation]** sources: modernc.org/sqlite README (official mirror): Pure-Go no-cgo driver; ships transpiled sqlite-vec extension in vec/ (https://github.com/modernc-org/sqlite)
- **[research_citation]** sources.omitted: 20 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: BOUNDARY VERIFICATION (primary sources): CD-0002 storage law scoped to Concord's own state (docs/decisions/CD-0002-concord-state-authority.md:19-31; Postgres named buy-fallback); core-architecture.md:40-60 Go scoped to the Concord core, :85-91 Rust explicitly available; vertical-integration.md:64 'Tools stay independent', :130-158 promotion contract crosses manifest-path + sha256, opaque, no runtime event; zero 'episode' references in concord *.go (grep); live concord#46 Decision (2026-08-30, operator approved): C20 resolved 'episode stays external, optional, and Product-scoped when configured', probe demoted to reopen trigger. The core structural argument verifies independently. Deviation from reference architecture: NONE.

CLAIM TABLE:
C1a 'modernc.org/sqlite cannot load C extensions' — HOLDS. No portable runtime dlopen: modernc libc Xdlopen/XLoadLibraryW abort on darwin/windows (https://github.com/go-again/sqlite/blob/v0.8.0/extension.go).
C1b corollary 'a Go rewrite wanting sqlite-vec would be constrained to the cgo driver' — REFUTED. modernc.org/sqlite now ships sqlite-vec transpiled in vec/ (https://pkg.go.dev/modernc.org/sqlite/vec v1.57.0; README 'the sqlite-vec extension in vec/'); blank import auto-registers vec0 (https://pkg.go.dev/gosqlite.org/vec; go-again/sqlite dev/coverage/vec.md). sqlite-vec's official Go docs also document a WASM path with cgo-free ncruces/go-sqlite3 (https://github.com/asg017/sqlite-vec/blob/main/site/using/go.md; staleness caveat https://github.com/ncruces/go-sqlite3/discussions/121). sqlite-vec reaches Go through three paths, two without cgo.
C2 'rusqlite supports load_extension_enable + LoadExtensionGuard' — HOLDS, imprecise as worded: requires the non-default 'load_extension' crate feature (https://docs.rs/rusqlite/latest/rusqlite/struct.LoadExtensionGuard.html).
C3 'mattn/go-sqlite3 supports extension loading' — HOLDS, and 'enabled by default' is exact: 'Loading external extensions is enabled by default. To disable... sqlite_omit_load_extension' (https://github.com/mattn/go-sqlite3/wiki/Features).
C4a 'vec0 write amplification is a property of its storage layout, runtime-independent' — HOLDS. Chunked storage + validity bitmaps, default chunk_size 1024 (sqlite-vec.c); deleted space reclaimed only in latest chunk (issue #184, author); no vacuum/optimize (issue #185); production: 'DELETE only sets a validity bit without reclaiming storage. Our database grows monotonically with every re-embedding' (issue #259). Nuance: episode is append-mostly with idempotent dedup, so exposure is workload-dependent — the record itself says this.
C4b 'sqlite-vec younger/less mature than pgvector' — HOLDS. sqlite-vec pre-v1, 'expect breaking changes' (https://alexgarcia.xyz/sqlite-vec/api-reference.html); v0.1.7-alpha.9 (2026-02, issue #261); INSERT OR REPLACE only landed 2026-04 (commit b95c05b); vec0 KNN is exhaustive — ANN (IVF/DiskANN) experimental behind compile flags. pgvector: 0.8.0 (2024-10-30, iterative scans) through 0.8.6 (2026-07-29) (CHANGELOG). Note: pgvector is also 0.x; its maturity is cadence, adoption, and ANN depth, not a 1.0 badge.

STRONGEST CASE AGAINST: (1) The trigger is inert, not merely delayed — blocked on three independent counts: episode disabled on the host (concord#46 comment 2026-08-14); addProductScopingRecall does not exist in the episode repository (comment 2026-08-28; spec 0010 explicitly defers automatic product scoping and the probe); the probe's ingestion source retires with the predecessor, so its window is finite (comment 2026-08-14, tracked as concord#101). A conditioned option whose condition cannot be evaluated is a dangling option; AC3 demands otherwise. (2) Option decay is real and unbounded — porting cost doubled in 19 days (~1500 to 3225 lines); every Tier 1-3 capability raises the eventual rewrite price; deferring dominates only if the probe can produce evidence in bounded time, which it currently cannot. If the window closes before re-enable + implementation, the decision silently degrades to 'never revisit', which was never decided. (3) Deployment cost is undercounted as a 'Vision-proxy deployment shape' note — Postgres is a second daemon (container, volume, healthcheck, pool, DATABASE_URL), and the service it serves is switched OFF today; the record asserts 'operational, not implementation' with no recorded cause, so friction is neither confirmed nor excluded. Counterweight: the boundary facts verify independently; the deployment benefit of SQLite is unmeasured; and the only in-scope SQLite vector engine is pre-v1 alpha with layout-inherent write amplification against pgvector's steady releases and HNSW ANN. Direction holds; the trigger section of the record does not.

THIRD OPTION (Rust + SQLite): a real gap in the alternatives section, and the only option that addresses the deployment attack below the cost of a language rewrite. Keeps rmcp, fastembed, ingest, scheduler; only store.rs, migrations, config change. Capability verified: rusqlite load_extension + sqlite-vec (C2, feature-gated); brute-force cosine over BLOBs viable at episode's scale (magic-context pattern; thousands of rows at 1024d is a tens-of-ms warm scan). It is already half-present: the trigger's measurement plan lists 'brute-force fp32 cosine in Rust' and 'sqlite-vec vec0' as arms — but never as a present-day alternative. Why it still loses today: vec0 KNN is exhaustive vs HNSW; JSONB+GIN metadata filtering (migration 0002, spec 0010 recall filters) has no equivalent — vec0 metadata columns are documented slower on full scans and inefficient with strings over 12 chars; sqlite-vec is pre-v1 alpha expecting breaking changes with delete-does-not-reclaim write amplification; Postgres is already deployed and verified end-to-end; and the deployment pain it would solve is unmeasured. Verdict on the omission: real gap — must be recorded and rejected on those grounds; does not flip the recommendation.

WHAT NEARLY BROKE IT: the trigger. If the ingestion window closes before re-enable + probe implementation, the conditioned follow-up becomes permanently unfireable — a dangling option wearing a trigger's clothes. The fix is precondition-and-deadline language, not a direction change.
- **[unresolved_action]** required_main_agent_actions: Correct the six blocking findings before acceptance.
- **[unresolved_action]** required_main_agent_actions: Reconcile the 2026-08-30 C20 decision with Concord's binding clarification document.
- **[unresolved_action]** required_main_agent_actions: Keep all verified episode and Concord runtime citations unchanged.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] When a contract names identity as path plus digest, do not invent a string separator. Preserve separate fields unless the owner defines canonical serialization.
- **[archive_only_evidence]** verification: tests_run=oc-fresh status --repo /home/jon/dev/episode --json, wc -l src/*.rs, source and document inspection in /home/jon/dev/episode and /home/jon/dev/concord, gh issue view 46 -R Sharper-Flow/concord --json body,comments, gh issue view 101 -R Sharper-Flow/concord --json body,comments, gh pr view 587 -R Sharper-Flow/concord --json commits,mergedAt, gh release view --repo asg017/sqlite-vec --json tagName,publishedAt,url results=n/a — Read-only citation audit. No repository changes or executable behavior required tests. Episode and Concord working trees remained clean.
- **[unresolved_action]** required_main_agent_actions: Treat the nine requested proposal checks as passed.
- **[unresolved_action]** required_main_agent_actions: Reconcile concord#46's issue body with accepted docs/clarifications.md C20, or record evidence for a real direction change before citing the issue body.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] A GitHub issue body can drift from accepted repository law. Verify decision claims against committed clarification or decision records and inspect issue comments separately.
- **[archive_only_evidence]** verification: tests_run= results=n/a — Read all four proposals through adv_change_show artifact-only projection. Verified CD-0002:104-109, clarifications C8/C20, vertical-integration promotion contract, CD-0026 D1/D3, lesson dispatch and publication code, GitHub issues #101/#46, and sqlite-vec releases/docs. No files changed.
- **[epic_terminal_note]** epic.membership: shapeEpisodeStructuredMemory · Storage/language direction decision (order 1)

## Contract / AC Coverage

No contract items.

## Unresolved Actions

- Correct the six blocking findings before acceptance.
- Reconcile the 2026-08-30 C20 decision with Concord's binding clarification document.
- Keep all verified episode and Concord runtime citations unchanged.
- Treat the nine requested proposal checks as passed.
- Reconcile concord#46's issue body with accepted docs/clarifications.md C20, or record evidence for a real direction change before citing the issue body.
