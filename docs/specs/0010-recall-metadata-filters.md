# rq-episode-recall-metadata-filters01

## Statement

Recall may constrain semantic results by structured memory context while excluding unresolved follow-ups from default agent feed-forward.

## Filter Contract

- `product` and `work_id` match exact metadata values.
- Every requested tag must be present; tag order and additional stored tags do not matter.
- A memory matches any requested `kind` from the typed column.
- Product, work ID, tags, kind, and namespace compose with AND semantics.
- Empty strings, empty lists, and blank list members reject before embedding.
- Omitted fields add no constraint.

## Open Follow-ups

Default recall excludes `metadata.action.kind = open_followup` while retaining actionless, resolved, and linked memories. `include_open_followups: true` explicitly includes unresolved rows that satisfy all remaining filters.

## Query and Index Safety

All caller values are bound with sqlx `QueryBuilder`; caller text never enters SQL syntax. Positive product/work/tag containment uses `metadata @>` and `memories_metadata_gin` with `jsonb_path_ops`. The index claim applies to positive containment, not the negative open-follow-up predicate.

Migration failure or checksum mismatch aborts startup before MCP serving. Namespace expansion, cosine ordering, and top-k clamping remain unchanged.

## Boundaries

Automatic product scoping and the C8 probe remain separate. This capability does not add promotion, recency, ranking, ingest-quality, host-capture, typed-column, Concord, or Fleet behavior. Fleet remains intentionally disabled.

## Verification

DB-free tests prove validation, schema, and bound SQL structure. Model-free Postgres integration proves each dimension, composition, open-follow-up behavior, migration/index presence, and GIN planner eligibility.
