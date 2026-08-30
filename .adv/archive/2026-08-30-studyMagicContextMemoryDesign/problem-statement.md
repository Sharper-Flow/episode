## Problem

Research done from `toolbox` on 2026-08-29 identified `cortexkit/magic-context` (MIT, npm `@cortexkit/opencode-magic-context`) as a close design analogue to `episode`. The findings are recorded in this change's proposal, but they are not attached to the sibling changes whose decisions they affect. Research that is not routed decays: `decideStorageLanguageDirection` will be decided without this evidence, and `addPromotionStateMemories` will be specified without the model-pinning caveat.

## Scope

This is a research-routing change. It produces artifact edits on sibling ADV changes and one verification result. It commits to no implementation in `episode` and touches no `episode` source.

**In scope**
- Read the UNVERIFIED `ARCHITECTURE.md` from `cortexkit/magic-context`, since the proposal blocks ideas 1 and 4 on it.
- Route evidence bearing on the storage direction into `decideStorageLanguageDirection`.
- Record the consolidation caveat on `addPromotionStateMemories`: a consolidation pass must pin its model and fail closed, never fall back to the caller's session model.
- Route the hybrid lexical/dense retrieval finding into `improveRecallQualityIngest` as a bound candidate.
- Record ideas 3 (tiered config with unsafe-field stripping), 5 (384d vs 1024d embedding tradeoff), and 6 (`harness` column) as known options requiring no action.
- Record the negative result for idea 2 so it is not re-investigated.
- Record the anti-recommendation: prompt-prefix rewriting is not a technique to copy.

**Out of scope**
- Deciding the storage direction. That decision belongs to `decideStorageLanguageDirection`.
- Any change to `episode` source, schema, or dependencies.
- Building a consolidation pass or adopting hybrid retrieval.

## Resolved before the proposal gate

Idea 2 hypothesized that `episode` fragments one project's memory across ADV per-change worktrees. It does not. `src/config.rs:147-191` parses `EPISODE_PROJECT_ROOTS` as explicit operator-declared `namespace=path` entries. `src/server.rs` takes `namespace` as a caller-supplied parameter defaulting to `global`. No `cwd` or git-root derivation exists anywhere in the scope path. The contingent defect change is not opened. The negative result is itself worth recording.

## Success

1. `ARCHITECTURE.md` is read, and any finding that changes idea 1 or 4 is folded in before routing.
2. `decideStorageLanguageDirection` carries the magic-context storage evidence with its correct disposition, plus a concrete measurement plan bound to the existing rewrite trigger. Discovery inverted the original expectation here: the evidence corroborates the sibling's standing "stay Rust + Postgres" recommendation rather than adding a competing option, so the routed content is corroboration plus a trigger-time measurement plan, not a new option.
3. `addPromotionStateMemories` carries the pinned-model, fail-closed caveat, scoped to the roles that actually exhibit the fallback.
4. `improveRecallQualityIngest` carries the hybrid-retrieval candidate, bound by its own AC3 calibration bar.
5. Ideas 2, 3, 5, and 6 are recorded with their dispositions, including the idea-2 negative result.
6. No `episode` source file is modified by this change.
