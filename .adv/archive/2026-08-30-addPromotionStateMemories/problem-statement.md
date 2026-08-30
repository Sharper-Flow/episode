# Problem statement

## What is wrong

Episode stores agent memories but has no first-class model of what happens when a memory stops being episodic and becomes durable knowledge.

The raw material for that model already exists in the source data. ADV wisdom entries carry `promoted_at` and `invalidated_by`, and `ingest.rs:68` already skips entries marked `invalidated_by`. Episode reads those fields and acts on one of them, but models neither.

Three capabilities are missing:

1. **No way to mark a promotion candidate.** A lesson that keeps recurring across work has no way to be flagged as worth graduating into durable knowledge.
2. **No way to record that a memory graduated, or where it landed.** There is no link from an episode memory to the spec or decision that now owns its content.
3. **No way to exclude graduated memories from recall.** A memory whose content now lives in a formalized Concord record still surfaces from episode.

## Why it matters

Without this, episodic memory and durable Product knowledge drift apart and duplicate each other. Two failure modes follow, and both are silent:

- A lesson graduates into a spec, but episode keeps surfacing the episodic copy. Agents get the same knowledge from two sources that can now disagree, with no signal about which one is law.
- A lesson never graduates at all. It ages out of relevance inside episode while never reaching the durable record that would have preserved it.

This is the seam the epic exists to build. Episode owns episodic memory. Concord owns Product law. Nothing currently connects the two, so knowledge either duplicates across the boundary or falls through it.

## Why now

The receiving side exists as of 2026-08-30. Concord #587 landed the promotion-receiving contract at `docs/vertical-integration.md:130-158`, satisfying concord#46 Item 2. It specifies target identity, back-link, acknowledgement, and recall exclusion on Concord's side.

Before that contract, building the episode half would have meant emitting into a void. The contract now names what a target is, and explicitly assigns recall exclusion to episode. This change implements episode's side of a defined seam rather than guessing at one.

Both declared prerequisites have also shipped: `RememberParams.context` and `RecallParams.filters` exist in `src/server.rs`, with spec `0010` and `tests/recall_filters_it.rs` on disk. Nothing blocks this work.

## Known operating condition

Episode is currently not running. The deployed binary at `~/.local/bin/episode` was built 2026-07-10 and predates migration `0002_metadata_gin.sql`, which landed 2026-08-11. The store at port 5434 records both migrations as applied, so `sqlx` refuses to start against a binary that embeds only the first. The service was disabled 2026-08-19 and the `memories` table holds zero rows.

This is recorded as a deliberate condition of the work, not an oversight. The change is implementable and testable against a local store regardless. Restoring the service is operational work outside this epic, and it must happen before any of this capability is exercised in practice.

## What is out of scope

- Auto-promotion heuristics. Manual flagging comes first; heuristic promotion stays a recorded later candidate.
- Anything on Concord's side. The receiving contract is delivered and unchanged by this work.
- Restoring the episode service.
