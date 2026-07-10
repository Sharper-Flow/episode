-- episode: agent decision-memory store schema.
-- Requires the pgvector extension (provided by the pgvector/pgvector image).

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS memories (
    -- Stable row id. For ingested rows this mirrors the ADV source id
    -- (pw-* / rf-*); for manual `remember` writes it is mem-*.
    id          TEXT PRIMARY KEY,

    -- Project namespace (e.g. 'pokeedge', 'pokeedge-web', 'advance') or 'global'
    -- for manually-remembered, cross-project memories.
    namespace   TEXT NOT NULL,

    -- Provenance of the memory.
    --   'adv_wisdom'     -> ingested from {project}/.adv/wisdom.jsonl
    --   'adv_reflection' -> ingested from {project}/.adv/reflections.jsonl
    --   'manual'         -> written via the `remember` MCP tool
    source      TEXT NOT NULL,

    -- Original ADV id for ingested rows (used for idempotent dedup). NULL for manual.
    source_id   TEXT,

    -- Free-form subtype: wisdom type (pattern|gotcha|convention|success|failure)
    -- or reflection facet (friction|highlight|suggestion). NULL allowed.
    kind        TEXT,

    -- The text that was embedded and is returned on recall.
    content     TEXT NOT NULL,

    -- Structured provenance: source_change, source_task, change_id, tags,
    -- origin_repo_*, etc. Kept as JSONB for flexible filtering later.
    metadata    JSONB NOT NULL DEFAULT '{}',

    -- Embedding vector. 1024 dims fits both fastembed (BGE-family) and
    -- voyage-4-lite, so the backend can switch without a schema migration.
    embedding   vector(1024) NOT NULL,

    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Idempotent ingestion: at most one row per (namespace, source, source_id).
-- Partial index so multiple manual rows (source_id IS NULL) are allowed.
CREATE UNIQUE INDEX IF NOT EXISTS memories_source_uniq
    ON memories (namespace, source, source_id)
    WHERE source_id IS NOT NULL;

-- Approximate nearest-neighbour recall via HNSW + cosine distance.
CREATE INDEX IF NOT EXISTS memories_embedding_hnsw
    ON memories USING hnsw (embedding vector_cosine_ops);

-- Namespace scoping for recall (project + global).
CREATE INDEX IF NOT EXISTS memories_namespace_idx
    ON memories (namespace);
