CREATE INDEX IF NOT EXISTS memories_metadata_gin
    ON memories USING gin (metadata jsonb_path_ops);
