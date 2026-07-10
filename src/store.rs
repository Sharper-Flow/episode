//! Postgres + pgvector persistence.

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::types::{MemoryInput, NamespaceStat, RecallHit};

/// Thin handle over a connection pool. Cheap to clone (Arc inside `PgPool`).
#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    /// Connect to Postgres and return a pool handle.
    ///
    /// NOTE: does NOT run `sqlx::migrate!("./migrations")` — the `migrate`
    /// feature is not enabled in `Cargo.toml` (sqlx is built with
    /// `default-features = false` and a feature list that omits `migrate`),
    /// and this task is constrained to editing only `src/store.rs`. Migrations
    /// in `migrations/0001_init.sql` are idempotent (`IF NOT EXISTS`) and are
    /// expected to be applied out-of-band (the live DB already has them).
    /// To restore auto-migration, add `migrate` to the sqlx features and
    /// re-add `sqlx::migrate!("./migrations").run(&pool).await?;` here.
    pub async fn connect(database_url: &str, pool_size: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(pool_size)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    /// Idempotent upsert keyed on stable `id`. Returns true on insert/update.
    pub async fn upsert(&self, input: &MemoryInput, embedding: &[f32]) -> Result<bool> {
        let vector = pgvector::Vector::from(embedding.to_vec());
        sqlx::query(
            "INSERT INTO memories (id, namespace, source, source_id, kind, content, metadata, embedding) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (id) DO UPDATE SET \
                 namespace = EXCLUDED.namespace, \
                 source = EXCLUDED.source, \
                 source_id = EXCLUDED.source_id, \
                 kind = EXCLUDED.kind, \
                 content = EXCLUDED.content, \
                 metadata = EXCLUDED.metadata, \
                 embedding = EXCLUDED.embedding, \
                 updated_at = now()",
        )
        .bind(&input.id)
        .bind(&input.namespace)
        .bind(input.source.as_str())
        .bind(&input.source_id)
        .bind(&input.kind)
        .bind(&input.content)
        .bind(&input.metadata)
        .bind(&vector)
        .execute(&self.pool)
        .await?;
        Ok(true)
    }

    /// Fast dedup check: does a row exist for (namespace, source_id)?
    pub async fn exists_source(&self, namespace: &str, source_id: &str) -> Result<bool> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM memories WHERE namespace = $1 AND source_id = $2)",
        )
        .bind(namespace)
        .bind(source_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(exists)
    }

    /// Cosine recall. `score = 1.0 - (embedding <=> query)` (cosine distance -> similarity).
    pub async fn recall(
        &self,
        query_embedding: &[f32],
        namespaces: &[String],
        top_k: i64,
    ) -> Result<Vec<RecallHit>> {
        let q = pgvector::Vector::from(query_embedding.to_vec());

        // Two separate `&'static str` query strings: sqlx 0.9 rejects dynamic
        // `String` SQL via `query()` (`SqlSafeStr`), so the namespace-filtered
        // and unfiltered variants are written out literally rather than built
        // with `format!`.
        let rows = if namespaces.is_empty() {
            sqlx::query(
                "SELECT id, namespace, source, kind, content, metadata, \
                     (1.0 - (embedding <=> $1))::float8 AS score \
                 FROM memories \
                 ORDER BY embedding <=> $1 \
                 LIMIT $2",
            )
            .bind(&q)
            .bind(top_k)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT id, namespace, source, kind, content, metadata, \
                     (1.0 - (embedding <=> $1))::float8 AS score \
                 FROM memories \
                 WHERE namespace = ANY($2) \
                 ORDER BY embedding <=> $1 \
                 LIMIT $3",
            )
            .bind(&q)
            .bind(namespaces)
            .bind(top_k)
            .fetch_all(&self.pool)
            .await?
        };

        let hits = rows
            .iter()
            .map(|row| RecallHit {
                id: row.get::<String, _>("id"),
                namespace: row.get::<String, _>("namespace"),
                source: row.get::<String, _>("source"),
                kind: row.get::<Option<String>, _>("kind"),
                content: row.get::<String, _>("content"),
                metadata: row.get::<serde_json::Value, _>("metadata"),
                score: row.get::<f64, _>("score") as f32,
            })
            .collect();
        Ok(hits)
    }

    /// Delete by id; return rows affected.
    pub async fn forget(&self, id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM memories WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Grouped counts per (namespace, source), ordered by namespace then source.
    pub async fn stats(&self) -> Result<Vec<NamespaceStat>> {
        let rows = sqlx::query(
            "SELECT namespace, source, count(*)::bigint AS count \
             FROM memories \
             GROUP BY namespace, source \
             ORDER BY namespace, source",
        )
        .fetch_all(&self.pool)
        .await?;

        let stats = rows
            .iter()
            .map(|row| NamespaceStat {
                namespace: row.get::<String, _>("namespace"),
                source: row.get::<String, _>("source"),
                count: row.get::<i64, _>("count"),
            })
            .collect();
        Ok(stats)
    }
}
