//! Postgres + pgvector persistence.

use anyhow::Result;
use sqlx::PgPool;

use crate::types::{MemoryInput, NamespaceStat, RecallHit};

/// Thin handle over a connection pool. Cheap to clone (Arc inside `PgPool`).
#[derive(Clone)]
pub struct Store {
    #[allow(dead_code)]
    pool: PgPool,
}

impl Store {
    /// WORKER A — implement:
    ///   - `PgPoolOptions::new().max_connections(pool_size).connect(database_url)`.
    ///   - run migrations: `sqlx::migrate!("./migrations").run(&pool).await`.
    ///   - return `Self { pool }`.
    pub async fn connect(_database_url: &str, _pool_size: u32) -> Result<Self> {
        todo!("WORKER A: connect pool + run migrations")
    }

    /// WORKER A — idempotent upsert.
    ///   - bind embedding via `pgvector::Vector::from(embedding.to_vec())`.
    ///   - ingested rows (source_id set): `ON CONFLICT (namespace, source, source_id)
    ///     WHERE source_id IS NOT NULL DO UPDATE SET content, kind, metadata, embedding,
    ///     updated_at = now()`. Manual rows (source_id NULL): `ON CONFLICT (id) DO UPDATE`.
    ///   - `source` column stores `input.source.as_str()`.
    ///   - return true when a row was inserted or updated.
    pub async fn upsert(&self, _input: &MemoryInput, _embedding: &[f32]) -> Result<bool> {
        todo!("WORKER A: upsert with pgvector embedding")
    }

    /// WORKER A — fast dedup check: does a row exist for (namespace, source_id)?
    pub async fn exists_source(&self, _namespace: &str, _source_id: &str) -> Result<bool> {
        todo!("WORKER A: SELECT EXISTS(...)")
    }

    /// WORKER A — cosine recall.
    ///   - if `namespaces` non-empty: `WHERE namespace = ANY($namespaces)`.
    ///   - `ORDER BY embedding <=> $query LIMIT $top_k`.
    ///   - `score = 1.0 - (embedding <=> $query)` (cosine distance -> similarity).
    ///   - map rows to `RecallHit`.
    pub async fn recall(
        &self,
        _query_embedding: &[f32],
        _namespaces: &[String],
        _top_k: i64,
    ) -> Result<Vec<RecallHit>> {
        todo!("WORKER A: HNSW cosine recall")
    }

    /// WORKER A — delete by id; return rows affected.
    pub async fn forget(&self, _id: &str) -> Result<u64> {
        todo!("WORKER A: DELETE ... RETURNING rows_affected")
    }

    /// WORKER A — `SELECT namespace, source, count(*) GROUP BY 1,2 ORDER BY 1,2`.
    pub async fn stats(&self) -> Result<Vec<NamespaceStat>> {
        todo!("WORKER A: grouped counts")
    }
}
