# Outcome — magic-context research, routed

Research on [`cortexkit/magic-context`](https://github.com/cortexkit/magic-context) (MIT), originally captured from `toolbox` on 2026-08-29, has been verified and routed into the sibling changes it affects. Nothing was built. Source basis: HEAD `a54f9c0`.

## What changed about the research itself

The originating proposal ranked its seven ideas. Verification inverted the top two, so the routing does not match the proposal's own priority order.

| Idea | Proposal's expectation | Verified disposition |
|---|---|---|
| 1. SQLite, no native module | "the single highest-value input" | **Weakened.** Corroborates staying on pgvector. |
| 4. Background consolidation | Adjacent, worth recording | **Strengthened.** Concrete validated mechanics. |
| 7. Hybrid lexical/dense retrieval | Not present | **New.** Found during verification. |

## Where each idea landed

**Idea 1 — SQLite storage → `decideStorageLanguageDirection`.** Routed as corroboration, not as a third option. magic-context runs exact brute-force fp32 cosine over SQLite BLOBs with no ANN index, designed for hundreds to ~1000 memories at 384 dimensions. Their own `MEMORY-DESIGN.md` names ANN as the escape hatch past that point — the path `episode` already occupies. Their ~10 ms figure is a full-scan number and is not comparable to an HNSW lookup.

Correction carried with it: their sqlite-vec rejection has two parts that transfer differently. "`bun:sqlite` can't load extensions" is runtime-specific and does not transfer (`rusqlite` and `mattn/go-sqlite3` both load extensions; pure-Go `modernc.org/sqlite` does not). "Write-amplification" is a `vec0` storage property and remains a live objection. A trigger-time measurement plan was recorded so AC3's conditioned follow-up has a concrete first action.

**Idea 4 — consolidation → `addPromotionStateMemories`.** Routed as evidence for that change's already-deferred auto-promotion item, without pulling it into scope. Transferable mechanics: two-axis promotion (cosine ≥ 0.85 clustering with 0.02 hysteresis, promoting when a cluster is confirmed on ≥ 2 distinct days *and* those confirmations span ≥ 7 elapsed days); exact content-hash dedup rather than semantic dedup; keep-versus-discard gated on whether a memory's backing file changed since `verified_at`, which `episode` can adopt directly since every ingested memory has one; lease domains so a background pass cannot race live traffic.

**Idea 7 — hybrid retrieval → `improveRecallQualityIngest`.** New finding, approved at the discovery checkpoint. Recorded as candidate sub-part 3d, bound by that change's AC3 calibration bar — a hybrid ranker is a blended score, so recording it respects AC3 while adopting it would not.

## Dispositions requiring no action

**Idea 2 — worktree memory fragmentation: RESOLVED NEGATIVE.** The research hypothesized that `episode` fragments a project's memory across ADV per-change worktrees. It does not. Namespaces are operator-declared `namespace=path` entries (`src/config.rs:147-191`), and `recall`/`remember` take `namespace` as a caller-supplied parameter defaulting to `global` (`src/server.rs`). No `cwd` or git-root derivation exists in the scope path. No defect change was opened. Recorded explicitly because the hypothesis was plausible enough to be raised again otherwise.

**Idea 3 — tiered config with unsafe-field stripping.** Applies only if `episode` grows project-local configuration. It has none: all configuration is environment-driven (`src/config.rs`), and environment variables are already a trusted-operator surface.

**Idea 5 — embedding size.** 384 dimensions against BGE-large at 1024. Not a defect on either side. Coupled to idea 1: it matters only if a brute-force direction is revisited, which idea 1's routing argues against.

**Idea 6 — cross-harness `harness` column.** `episode` is ADV/OpenCode-specific by design. A known generalization path, not a recommendation.

**Anti-recommendation, retained.** magic-context's prompt-prefix rewriting via `experimental.chat.messages.transform` is not a technique to copy. Toolbox ADR 0013 measured roughly 15 quota tokens burned per context token saved, because rewriting cached prefix content forces re-billing as `cache_creation`. This does not apply to `episode`, which is an MCP server and never touches the prompt array. Recorded so nobody later mistakes it for a validated technique.

**Two designs worth naming, no target.** Persist an embedding only if the content hash still matches after the provider call (`saveEmbeddingIfHashMatches`), and never run embedding inference inside a write transaction. Both are cheap correctness properties if `episode` ever moves embedding off the synchronous path.

One schema detail worth keeping: their `memory_embeddings` table has `PRIMARY KEY(memory_id, model_id)`, storing one vector per memory per embedding model. A model change adds vectors alongside the old ones instead of destroying them. That is the shape that makes an embedding-model migration survivable, and `episode`'s single-vector-per-row schema does not have it.

## The process finding, which is the durable one

Six factual errors entered this research. All six were corrected before the artifacts were accepted, but the pattern of how they were caught is the finding.

| Error | Nature | Caught by |
|---|---|---|
| FTS5 tokenizer preserves `. _ / -` | False claim from an upstream code comment | Design validator |
| Session-model fallback is general | True for `historian` only | Design validator |
| Replacement tokenizer DDL quote | The validator's own "verbatim" fix quoted a string absent from the source | Acceptance reviewer, then the source |
| `memory_embeddings` DDL | Omitted `model_id` and the composite key | Acceptance reviewer |
| Promotion thresholds | Swapped: 2 is distinct-day support, 7 is elapsed span | Acceptance reviewer |
| Lease key attribution | Cited `lease.ts:8`; assignment is in `dreamer/task-registry.ts` | Acceptance reviewer |

A seventh defect was internal rather than a citation: the storage section called both systems "full fp32 scans" while the same section stated `episode` occupies the ANN escape path. The acceptance reviewer caught the contradiction.

Three things stand out.

First, no error was caught by the agent that wrote it. Every one required an independent reader.

Second, the design validator introduced an error while fixing one. Its corrected tokenizer DDL was itself fabricated, and survived until the acceptance reviewer disagreed and the orchestrator fetched the file. A single reviewing pass would have shipped invented SQL into a decision record with higher confidence than the original error carried, because it arrived labelled as a verification result.

Third, this summary undercounted its own errors at four until the harden pass recounted them at six. The document about citation discipline was itself imprecise.

The operative lesson for a project building a memory system: a citation asserted by an agent is not evidence, whether that agent is writing or reviewing. Only fetching the source is. Where two agents disagree, neither verdict is authoritative.

## Residual UNVERIFIED

None of this blocks a routing decision.

- Full line-level diff between `a32a839` and `a54f9c0` outside the Memory, Dreamer, and Historian sections. Deltas elsewhere appeared additive.
- `render-mural` task registration — present in the documentation's lease list, absent from the `CANONICAL_DREAM_TASKS` array.
- Any committed benchmark output from `ctx-search-benchmark.ts`. The script exists; no results are committed.
- Whether the `bge-small-en-v1.5` provider named in `MEMORY-DESIGN.md` is implemented or aspirational.

## Follow-ups, not actioned here

Two ADV defects were observed during this change and belong to the Advance project, not `episode`:

- **Phase-plan budget starvation.** `adv_change_show include:{phasePlan:true}` truncates deterministically once gate approval evidence grows. On this change `_phasePlan` was 47921 chars against a 52017 budget, with the gates block pushing the total to 56468. The acceptance directive was unreadable, and no alternate route exists — `adv_tool_catalog` lists no directive tool. Thorough gate notes make the phase directive unreadable at exactly the late gates where it matters.
- **Spurious `filesTouched`.** Every completed task on this change lists six repo files it never wrote. Git is authoritative: zero commits, empty diff against `main`.

Neither was filed. `adv_change_create` in this deployment exposes no `status` parameter, so a backlog-status record cannot be created, and filing into the Advance project requires cross-project write approval that was not sought.

One further item was surfaced and left out of scope: mirroring these corrections into `episode` wisdom for cross-epic recall.

## Not done, deliberately

No `episode` source file was modified, proven by `runId tr_mtf91w8w_3456b934`: empty `git status --porcelain`, empty `git diff --stat main...HEAD`. This change commits to no implementation. The storage direction remains `decideStorageLanguageDirection`'s to decide, auto-promotion remains deferred, and hybrid retrieval remains a candidate.
