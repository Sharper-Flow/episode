-- A memory row's identity is (namespace, id), not id alone.
--
-- Ingested ids are raw per-project ADV ids (`pw-1`, `{rf_id}:{kind}:{index}`),
-- so two watched projects both hold `pw-1`. Under the single-column primary
-- key the second project's upsert hit `ON CONFLICT (id) DO UPDATE` and rewrote
-- the first project's row in place — namespace included. The unique index
-- `memories_source_uniq (namespace, source, source_id)` never evaluated: the
-- primary-key conflict won first.
--
-- The composite key makes the steal structurally impossible: two namespaces
-- cannot produce the same (namespace, id) pair.
ALTER TABLE memories DROP CONSTRAINT memories_pkey;
ALTER TABLE memories ADD PRIMARY KEY (namespace, id);

-- The composite primary key's btree leads on namespace, so it serves every
-- namespace-only predicate this index served.
DROP INDEX memories_namespace_idx;
