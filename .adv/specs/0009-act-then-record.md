# rq-episode-act-then-record01

## Statement

Given a caller-classified issue-sourced memory, when the caller records it, then its context identifies the issue as resolved ad hoc, linked to tracked work, or left as an open follow-up.

## Rationale

Episode stores lessons, not passive friction. An issue must not become an apparently completed lesson merely because it was recorded. The caller that observed the issue owns classification; Episode validates and preserves the supplied action state without inferring intent from free-form content or memory kind.

This contract is enabling infrastructure. It does not itself create the host capture surface or cause agents to record memories.

## Action Contract

Action is optional in `MemoryContext`. Conventions, decisions, and other non-issue-shaped memories remain valid without it.

The stable metadata path is `metadata.action.kind`. Supported wire shapes are exactly:

```json
{"kind":"ad_hoc_resolved","summary":"fixed the owning mechanism"}
{"kind":"linked_work"}
{"kind":"open_followup"}
```

- `ad_hoc_resolved` requires a summary containing non-whitespace content. Accepted summary bytes are preserved exactly.
- `linked_work` requires a non-empty, non-whitespace outer `MemoryContext.work_id`; the action does not duplicate that identifier.
- `open_followup` has no payload. It means the issue remains unresolved and is an operator signal, not a completed lesson.
- Unknown action kinds, unsupported payload fields, and invalid cross-field combinations reject before embedding or persistence.
- Omitted or null action remains actionless and emits no `action` metadata key.

## Ownership Boundary

The caller owns whether a memory is issue-shaped and must supply action state for issue-sourced capture. Episode validates supplied action only. It does not infer issue shape from `kind`, content, tags, repository, or Concord-specific work semantics.

A future OpenCode host capture surface such as `/record-lesson` remains required to turn this contract into routine corpus growth. That surface must prompt for or derive caller-owned action after the agent acts. Until it exists, this change must not be described as fixing capture adoption by itself.

## Recall Boundary

Recall filtering is a separate capability. `addRecallMetadataFiltersGin` must consume `metadata.action.kind` and exclude `open_followup` from default agent feed-forward recall. Open follow-ups may be included explicitly for operator triage.

Until that downstream behavior ships, this contract changes capture representation only; it does not change recall results.

## Operational Boundary

Fleet remains intentionally disabled. Do not re-enable it merely because the action-state type exists. Re-enablement requires a working act-then-record capture surface and demonstrated corpus payoff.

## Authority

- `src/types.rs` owns `ActionState`, `MemoryContext.action`, and cross-field validation.
- `src/server.rs` owns validation-before-embedding and MCP invalid-params classification.
- Existing JSONB metadata persistence remains unchanged.

## Verification

- Runtime tests cover all variants, sparse actionless capture, exact value preservation, malformed shapes, and cross-field rejection.
- Generated-schema tests independently prove exact variant names and closed variant branches.
- `cargo fmt --all --check`, clippy with warnings denied, and `cargo test --locked` must pass.

## Constraints

- No recall SQL, GIN index, migration, typed action column, issue inference, host-command implementation, Concord implementation, or Fleet re-enablement belongs to this capability change.
- Promotion state remains a separate metadata concern.
