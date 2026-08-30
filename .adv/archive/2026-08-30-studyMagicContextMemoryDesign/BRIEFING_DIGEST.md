# Archive Briefing Digest

**Change ID:** studyMagicContextMemoryDesign
**Title:** Study magic-context memory design
**Status:** archived
**Generated:** 2026-08-30T14:12:35.769Z

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
| release | done |

## Epic Context

No Epic membership

## Durable Facts

Showing 23 of 23 durable facts.

- **[report_follow_up]** follow_ups: Spawn prompt lacked explicit TASK_SCOPE/IN_SCOPE/OUT_OF_SCOPE/DONE_WHEN/STOP_WHEN/VERIFICATION anchor blocks; scope was taken from the Goal/Questions/Output sections (warning per packet contract).
- **[report_follow_up]** follow_ups: Spawn SCOPE KEY 'discovery-architecture-md' lacked the lane prefix required by the report schema; submitted as researcher:discovery-architecture-md.
- **[report_follow_up]** follow_ups: If exact a32a839 line numbers are needed for code files, re-fetch via raw.githubusercontent at that SHA; searchcode serves HEAD a54f9c0 only.
- **[research_citation]** sources: ARCHITECTURE.md at requested commit a32a839 (raw): Pinned-commit copy; Memory/Dreamer/Historian sections extracted and compared against HEAD — substantively identical. (https://raw.githubusercontent.com/cortexkit/magic-context/a32a839/ARCHITECTURE.md)
- **[research_citation]** sources: ARCHITECTURE.md at HEAD a54f9c0 (searchcode): Full 180-line read: Memory/search/embeddings (L111), Dreamer (L118), Historian flow (L94), Storage & migrations (L146), Error handling (L171). (https://github.com/cortexkit/magic-context/blob/a54f9c06e91407119c20f39dd89c08f9b2bad5ac/ARCHITECTURE.md)
- **[research_citation]** sources: MEMORY-DESIGN.md: Schema (memory_embeddings BLOB side table, L108-111), MiniLM 384d model + known weakness + hybrid FTS5 mitigation (L256-262), 1536B/vector, 1000 memories ≈ 1.5MB (L265-267), ANN-at-1000+ degradation note (L277), alternative providers incl. fastembed bge-small (L279-286). (https://github.com/cortexkit/magic-context/blob/main/packages/plugin/docs/MEMORY-DESIGN.md)
- **[research_citation]** sources.omitted: 8 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: magic-context's memory subsystem is an existence proof for SQLite-only semantic memory at small scale, and its own docs name the wall: brute-force exact cosine over in-memory Float32Array BLOBs, ~10ms/query on a 79-memory pool, ANN named only as a 1000+ contingency. episode already runs the growth path magic-context defers (pgvector HNSW, 1024d). For consolidation, magic-context supplies a mature, directly transferable pattern set: conflict-domain leases with BEGIN IMMEDIATE guarded writes, non-agentic LLM tasks emitting one fail-closed-parsed XML manifest applied host-side, exact-hash dedup at promotion, per-memory verify gates on backing-file change with sticky partial progress, cluster+span+occurrence promotion thresholds, and provider-output-failure detection. Code citations are from searchcode index HEAD a54f9c0; ARCHITECTURE.md was also diffed against the requested a32a839 via raw.githubusercontent (sections Memory/Dreamer/Historian substantively identical; a32a839 is 51.7KB vs HEAD 62.5KB, deltas additive in Overview/Pi-parity prose).
- **[report_follow_up]** follow_ups: Spawn-packet aux anchors (TASK_SCOPE/IN_SCOPE/OUT_OF_SCOPE/DONE_WHEN/STOP_WHEN/VERIFICATION blocks) were not present in verbatim form; scope was taken from the prompt's 'What to validate' sections. Identity anchors were all present.
- **[report_follow_up]** follow_ups: Residual unverified details (non-load-bearing): exact production migrations.ts memories_fts DDL variant (tests use plain 'porter unicode61'; docs use categories variant — no variant preserves . _ / - since 'tokenchars' is absent repo-wide); dreamer verify-task file-gating internals (verify-gate.ts exists with matching function names); 'memory:<project>' BEGIN IMMEDIATE atomicity wording corroborated by lease.ts:8 only; committed ctx-search-benchmark output; bge-small provider aspirational-vs-implemented (already listed in the design's residual UNVERIFIED).
- **[research_citation]** sources: magic-context ARCHITECTURE.md @ a54f9c0: sqlite-vec rejection quote ('bun:sqlite can't load extensions + write-amplification'); 'session-model last resort, historian-only' label; Float32Array cosine scan. (https://github.com/cortexkit/magic-context/blob/a54f9c06e91407119c20f39dd89c08f9b2bad5ac/ARCHITECTURE.md)
- **[research_citation]** sources: magic-context MEMORY-DESIGN.md @ a54f9c0: memory_embeddings DDL verbatim; memories_fts DDL tokenize='porter unicode61 categories "L* N* Co"' with preservation comment; 79-mem/0.1s, 1536B/vector, 1.5MB/1000, escape-hatch quote; model weakness + bge benchmark rec; no-inference-in-transaction note. (https://github.com/cortexkit/magic-context/blob/a54f9c06e91407119c20f39dd89c08f9b2bad5ac/packages/plugin/docs/MEMORY-DESIGN.md)
- **[research_citation]** sources: primer-clustering.ts: L5 0.85, L6 0.02, L7 PROMOTION_THRESHOLD=2, L8 MIN_SPAN_DAYS=7. (https://github.com/cortexkit/magic-context/blob/a54f9c06e91407119c20f39dd89c08f9b2bad5ac/packages/plugin/src/features/magic-context/primer-clustering.ts)
- **[research_citation]** sources.omitted: 7 additional sources omitted (bounded to first 3)
- **[archive_only_evidence]** architecture_assessment: All five priority source claims were checked against magic-context @ a54f9c0. Four verify (brute-force 19-line cosine; promotion constants 0.85/0.02/2/7 at L5-8; historian-only session-model fallback — the highest-risk narrowing is CORRECT; sqlite-vec rejection quote verbatim). The fifth (FTS5 tokenizer preserving . _ / -) is the source's documented intent, not its actual behavior: their config equals the unicode61 default and 'tokenchars' appears nowhere in the repo. Two author inferences: the 4KB/2.7x arithmetic is sound; the sqlite-vec transfer correction is right about extension loading (rusqlite and mattn/go-sqlite3 both load extensions) but overreaches by also disposing of the write-amplification half, which is runtime-independent. All three sibling boundaries are respected; one internal inconsistency exists between this change's problem statement/success criteria (SQLite option + measurement plan) and the design's inverted R1 routing (corroborate, no new option, no plan included).
- **[unresolved_action]** required_main_agent_actions: Do not complete acceptance while citation-1, citation-2, and citation-3 remain.
- **[unresolved_action]** required_main_agent_actions: Correct the three sibling sections and the matching design and executive-summary claims through the owning ADV workflow.
- **[unresolved_action]** required_main_agent_actions: Correct the lease citation and the full-scan overstatement.
- **[unresolved_action]** required_main_agent_actions: Run another pinned-commit citation review after the artifact corrections.
- **[wisdom_candidate]** wisdom_candidates: [gotcha] For source-routed research, verify quoted DDL and threshold semantics against the pinned commit. A correct conclusion does not make an incorrect source quote acceptable.
- **[archive_only_evidence]** verification: tests_run=adv_change_show studyMagicContextMemoryDesign with problemStatement, design, and executiveSummary, adv_change_show for decideStorageLanguageDirection, addPromotionStateMemories, and improveRecallQualityIngest, Pinned raw-source checks at cortexkit/magic-context a54f9c0, SQLite FTS5 unicode61 documentation check results=fail — Artifact reads completed. Pinned source verified cosine-similarity.ts, storage-db.ts, primer-clustering.ts, compartment-runner-historian.ts, task-registry.ts, lease.ts, sidekick/agent.ts, task-executor.ts, and SQLite FTS5 docs. Three load-bearing source claims were false as written.
- **[unresolved_action]** required_main_agent_actions: Use this READY report as acceptance-review evidence. No artifact remediation remains.
- **[archive_only_evidence]** verification: tests_run=git status --porcelain && git diff --stat, git -C /tmp/opencode/magic-context-a54f9c0-review2 rev-parse --short=7 HEAD results=pass — Reviewed all four artifact-only projections and source at a54f9c0. memory_embeddings includes model_id and PRIMARY KEY(memory_id, model_id) at storage-db.ts:1094-1102. Promotion constants and summary logic confirm support >=2 distinct UTC days and spanDays >=7 elapsed days at primer-clustering.ts:7-8,143-180. memories_fts uses tokenize='porter unicode61' with no categories or tokenchars at storage-db.ts:1359-1365. task-registry.ts:7-8 and 94-140 assigns lease domains and derives lease keys. Episode migrations/0001_init.sql:35-37 stores one embedding in each memory row, while lines 49-51 define the HNSW index. The three sibling artifacts retain their opening content, complete AC blocks, and Epic context before appended evidence. The process-finding table matches the established review history. Episode working tree stayed clean.

## Contract / AC Coverage

No contract items.

## Unresolved Actions

- Do not complete acceptance while citation-1, citation-2, and citation-3 remain.
- Correct the three sibling sections and the matching design and executive-summary claims through the owning ADV workflow.
- Correct the lease citation and the full-scan overstatement.
- Run another pinned-commit citation review after the artifact corrections.
- Use this READY report as acceptance-review evidence. No artifact remediation remains.
