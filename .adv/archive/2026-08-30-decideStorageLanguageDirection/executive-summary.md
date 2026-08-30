# Storage and language direction — outcome

## What was decided

Episode stays on **Rust + Postgres + pgvector** for all Tier 1–3 structured-memory work. A storage or language rewrite is recorded as a conditioned follow-up, not planned work.

## What changed about the decision

The recommendation was drafted on 2026-08-11 and the conclusion survived unchanged. Almost none of its reasoning did.

**The central argument was false.** The draft's decisive "case against" bullet held that Concord was pre-runtime, so aligning with it was speculative. Concord had a working runtime before the question was framed: a CLI routing launcher, session, and JSON commands, a SQLite `domain_events` log with projections, an agent dispatcher, and a Bubble Tea TUI. Its floor-readiness record shows 40 items satisfied, 1 out of scope, 0 outstanding. It was v0.10.1 on the drafting date and v4.9.0 at acceptance. The draft had misread "pre-replacement-readiness" in the README as "pre-runtime."

Recording the decision on that premise would have made it stale on the day it was written, because the premise described a temporary condition that had already expired.

**The replacement basis is durable.** Concord's storage law does not reach episode. CD-0002 scopes "no second store holds authoritative state" to Concord's own authority and names Postgres as Concord's own buy-fallback. The Go direction is scoped to "the Concord core," with Rust explicitly kept available. Concord's Go code contains zero references to episode, and `vertical-integration.md` states that tools stay independent. The promotion seam crosses a manifest-path and sha256 boundary that is language- and storage-agnostic by construction.

**A stronger alternative had been omitted.** Rust + SQLite — keeping the language, dropping the server — answers the deployment objection for the cost of `store.rs`, migrations, and config rather than a full rewrite. It is now recorded and rejected on evidence: exhaustive stable KNN against HNSW, no JSONB and GIN equivalent, `vec0` metadata columns documented slower on scans and inefficient past 12-character strings, a pre-v1 engine with layout-inherent write amplification, and unmeasured pain. It returns as a live option at the decay checkpoint.

**The trigger was overstated.** The draft implied an insufficient probe fires the rewrite. It does not. It reopens C20, the episode-ownership question. Only a C20 resolution toward Concord owning the memory territory brings storage and language back into scope.

## The weakest part, recorded rather than hidden

The trigger cannot be evaluated today. Three independent blockages, one time-limited:

1. Episode is disabled on the host, with no recorded cause, so no evidence accumulates.
2. The probe change is unimplemented, and spec 0010 explicitly defers it.
3. The evidence window is finite — episode's ingestion source is predecessor wisdom and reflection state, which retires with the predecessor.

A condition that cannot be evaluated is a dangling option by AC3's own letter. An option-decay checkpoint now re-decides the change if predecessor retirement arrives without probe evidence, or if the codebase passes a size threshold at which porting stops being tractable. Porting cost roughly doubled in 19 days, from ~1500 to 3225 lines.

## Findings that changed other work

- **`addPromotionStateMemories` is not blocked and its receiver exists.** Concord #587 landed the promotion-receiving contract on the day of this work, satisfying concord#46 Item 2. Its two declared prerequisites had shipped without leaving ADV change records.
- **`improveRecallQualityIngest` sub-part 3a is half-delivered.** `RecallFilters` already carries a first-class `kinds` field; `source` does not exist. Building 3a as written would have duplicated shipped behavior. AC1 was narrowed.
- **`addPromotionStateMemories` gained a design constraint.** Concord's contract resolves targets by manifest path and sha256 and names a record version, so a bare spec-id will not resolve. Concord specifies no serialized format, so the carrying representation is recorded as an open design question rather than guessed.
- **Concord's learning-capture surface exists.** CD-0026 is Accepted: lessons publish through `concord_work_compact.lesson_publish` into git-backed markdown plus a manifest record, and reflections are lessons carrying a `reflection` tag. This converts `improveRecallQualityIngest` 3c from speculative future-proofing into work with a second real consumer, and establishes that the abstraction must be parser-shaped rather than format-shaped.
- **Three proposals declared shipped work as open blockers.** `enrichRememberWithContext` and `addRecallMetadataFilters` are live in `src/server.rs` but hold no ADV change records.

## Verification

One independent validator and two review passes before archive, and one correction after it. Seven citation errors were corrected before archive. One inverted claim survived into the archived record and is corrected here.

## Concord's position, corrected after archive

The archived record originally stated that no operator approval resolving C20 existed. That was wrong, and it inverted a material fact about the counterparty.

Concord's operator approved the direction on 2026-08-30 and recorded it in concord#46: **episode stays external, optional, and Product-scoped when configured**, with the probe demoted to a reopen trigger. The issue body is a work order. Its documentation deliverables had not run, so Concord's accepted documents still read as open.

Reading the documents alone produced "not resolved." Reading the issue alone produced "resolved." Two reviewers reported their own surfaces accurately, and the conflict was resolved in the wrong direction — by deleting the claim rather than by establishing which surface leads.

The reconciliation landed as concord PR #596, executing concord#46's deliverables: C20 resolved in place, the probe recorded as a reopen trigger, and a new statement that Concord carries no required dependency on episode.

**The decision is unaffected.** It rests on scope — CD-0002's authority rule and Concord's Go direction do not reach satellite services — which holds regardless of how C20 resolves. Concord's approval corroborates the boundary rather than creating it.

The durable lesson concerns source conflict, not citation care. Two reliable reviewers disagreed because they read different surfaces of one system at a moment when those surfaces disagreed. The failure was resolving that by deletion instead of by asking which surface leads. A tracker recording an approved decision leads its own unexecuted documentation deliverables.

## Scope

No repository files changed. All work targets ADV proposal artifacts; the tree stayed clean at `fb7088c` across every checkpoint.
