# rq-episode-recall-metadata-filters01

## Statement

Recall may constrain semantic results by structured memory context while excluding unresolved follow-ups from default agent feed-forward.

## Filter Contract

- `work_id` matches the exact metadata value.
- `product` scopes by ownership, not membership: see Product Scope below.
- Every requested tag must be present; tag order and additional stored tags do not matter.
- A memory matches any requested `kind` from the typed column.
- A memory matches any requested `source` from the closed set `manual`, `adv_wisdom`, `adv_reflection`; unknown values reject at deserialization and an empty list rejects validation.
- `max_age_days` excludes rows whose first capture (`created_at`) is older than the cutoff. Zero rejects validation. The basis is deliberately `created_at`: write-once ingest never moves it, and promotion transitions touch only `updated_at`. A retracted-then-reinstated row gets a fresh `created_at` — reinstatement is a new capture of re-validated knowledge. Recency is a filter, never a ranking input; cosine order is untouched.
- Product, work ID, tags, kind, source, age, and namespace compose with AND semantics.
- Empty strings, empty lists, blank list members, and zero day counts reject before embedding.
- Omitted fields add no constraint.

## Product Scope

A Product filter means "this Product's memories plus the shared pool", not "only rows tagged with this Product". The predicate is `(NOT (metadata ? 'product') OR metadata->>'product' = X)`:

- A row tagged `product = X` matches.
- A row with no `product` key matches: ingested predecessor wisdom and unscoped context land in the shared pool by design, and the namespace constraint already separates projects.
- A row tagged `product = Y ≠ X` does not match, including rows stored in a shared/global namespace. A row that declared a Product made its claim.
- A row with an explicit JSON `product: null` does not match. `?` treats JSON null as a present key and `->>` yields SQL NULL for its value, so the row fails both arms. A null in a field that exists is a claim of no product, not an absent field.

Product is caller-supplied data, not a reserved key. Unlike `promotion_state` (spec 0011) — episode-owned state whose forgery hides rows and poisons transitions — a source-supplied product tag cannot cross the namespace arm and grants nothing that untagged absence did not already grant. Episode never derives Product membership: the caller resolves "this Product's projects" and passes namespaces plus the product filter.

## Query and Index Safety

All caller values are bound with sqlx `QueryBuilder`; caller text never enters SQL syntax. Positive work/tag containment uses `metadata @>` and `memories_metadata_gin` with `jsonb_path_ops`. The index claim applies to positive containment only. The product predicate is not servable by this index: `jsonb_path_ops` does not support key existence, and the extracted-text equality arm is not an indexed expression. The negative open-followup and promoted exclusions also filter without index assistance — the accepted scan shape shared by all three.

Migration failure or checksum mismatch aborts startup before MCP serving. Namespace expansion, cosine ordering, and top-k clamping remain unchanged.

The first migration on a populated database builds the GIN index synchronously and may delay startup while PostgreSQL holds the required table lock. Plan the first rollout during a suitable maintenance window for large corpora.

## Open Follow-ups

Default recall excludes `metadata.action.kind = open_followup` while retaining actionless, resolved, and linked memories. `include_open_followups: true` explicitly includes unresolved rows that satisfy all remaining filters.

## Boundaries

Automatic product scoping (deriving Product membership episode-side) remains out of scope: product tags are caller-supplied. The C8/C20 product-scoping probe is pre-registered on concord#46 (baseline, method, rubric) and runs as operational follow-up; its outcome lands on that issue. This capability does not add promotion, recency, ranking, ingest-quality, host-capture, typed-column, Concord, or Fleet behavior. Fleet remains intentionally disabled.

## Verification

DB-free tests prove validation, schema, and bound SQL structure. Model-free Postgres integration proves each dimension, composition, product-scope semantics (tagged match, untagged shared pool visible, wrong Product excluded, explicit null excluded), source filtering over all three provenances with union semantics, recency cutoffs against seeded `created_at` values, open-follow-up behavior, migration/index presence, and GIN planner eligibility.
