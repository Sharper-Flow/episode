//! Postgres + pgvector persistence.

use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::types::{MemoryInput, NamespaceStat, RecallHit};

/// Maximum time to wait when acquiring a connection from the pool (AC3).
const POOL_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
/// Idle connection lifetime before it is closed (AC3).
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Absolute maximum connection lifetime before it is recycled (AC3).
const POOL_MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);

/// Build the `PgPoolOptions` with the pinned, explicit lifecycle bounds (AC3).
///
/// These are set explicitly rather than relying on sqlx defaults so a dependency
/// upgrade cannot silently change pool semantics. Factored as a pure builder so
/// the configured bounds are unit-testable without a database.
pub(crate) fn pool_options(max_connections: u32) -> PgPoolOptions {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(POOL_ACQUIRE_TIMEOUT)
        .idle_timeout(POOL_IDLE_TIMEOUT)
        .max_lifetime(POOL_MAX_LIFETIME)
        .test_before_acquire(true)
}

/// Thin handle over a connection pool. Cheap to clone (Arc inside `PgPool`).
#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    /// Connect to Postgres, run migrations, and return a pool handle.
    ///
    /// Eager by design: opens the pool and runs migrations at startup so a bad
    /// configuration or unavailable database surfaces immediately, bounded by the
    /// pool's `acquire_timeout` (AC3).
    pub async fn connect(database_url: &str, pool_size: u32) -> Result<Self> {
        let pool = pool_options(pool_size).connect(database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
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

    /// Transactional batch upsert for the ingestion path (AC4 / DONT3 / design §4).
    ///
    /// Persists one bounded ingestion partition in a single
    /// `INSERT ... ON CONFLICT (id) DO UPDATE` statement wrapped in a transaction.
    /// The statement is built with `sqlx::QueryBuilder` value binds — never
    /// `format!` — so mutation SQL is never string-constructed from input. `items`
    /// and `embeddings` are parallel, equal-length slices (the caller validates
    /// this via [`crate::scheduler::validate_batch_output`]; the length check here
    /// is defense-in-depth). On any error the transaction rolls back, so the whole
    /// partition either lands or none of it does; failed partitions remain eligible
    /// for the next reconcile.
    ///
    /// Returns the number of rows affected (inserted or updated). An empty input
    /// returns `Ok(0)` without opening a transaction.
    pub async fn upsert_batch(
        &self,
        items: &[MemoryInput],
        embeddings: &[Vec<f32>],
    ) -> Result<usize> {
        anyhow::ensure!(
            items.len() == embeddings.len(),
            "upsert_batch length mismatch: {} items vs {} embeddings",
            items.len(),
            embeddings.len()
        );
        if items.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO memories \
             (id, namespace, source, source_id, kind, content, metadata, embedding) ",
        );
        qb.push_values(
            items.iter().zip(embeddings.iter()),
            |mut b, (input, emb)| {
                b.push_bind(&input.id)
                    .push_bind(&input.namespace)
                    .push_bind(input.source.as_str())
                    .push_bind(&input.source_id)
                    .push_bind(&input.kind)
                    .push_bind(&input.content)
                    .push_bind(&input.metadata)
                    .push_bind(pgvector::Vector::from(emb.clone()));
            },
        );
        qb.push(
            " ON CONFLICT (id) DO UPDATE SET \
                 namespace = EXCLUDED.namespace, \
                 source = EXCLUDED.source, \
                 source_id = EXCLUDED.source_id, \
                 kind = EXCLUDED.kind, \
                 content = EXCLUDED.content, \
                 metadata = EXCLUDED.metadata, \
                 embedding = EXCLUDED.embedding, \
                 updated_at = now()",
        );

        let result = qb.build().execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(result.rows_affected() as usize)
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

    /// Bulk dedup lookup for the ingestion path (AC4 / design §4).
    ///
    /// Returns the subset of `source_ids` already present in `namespace`, in one
    /// static-SQL round-trip (`source_id = ANY($2)`) instead of one `EXISTS` per
    /// item. An empty candidate list short-circuits to an empty set without a
    /// database call. Null `source_id` rows never match `ANY` against a non-null
    /// candidate array, so manual rows (no `source_id`) are unaffected. Match
    /// semantics mirror [`Self::exists_source`] (namespace + source_id only) so
    /// the bulk path is a drop-in replacement for the per-item path.
    pub async fn existing_source_ids(
        &self,
        namespace: &str,
        source_ids: &[String],
    ) -> Result<HashSet<String>> {
        let mut found = HashSet::new();
        if source_ids.is_empty() {
            return Ok(found);
        }
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT source_id FROM memories \
             WHERE namespace = $1 AND source_id = ANY($2)",
        )
        .bind(namespace)
        .bind(source_ids)
        .fetch_all(&self.pool)
        .await?;
        found.extend(rows);
        Ok(found)
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

    /// Restricted hard deletion of a manual memory (AC1 / C3 / DONT1).
    ///
    /// Removes at most one row whose `id`, `namespace`, and `source = 'manual'`
    /// all match, and returns rows affected (`0` or `1`). Enforcement is
    /// structural: the `source = 'manual'` predicate lives in this single SQL
    /// statement, not in a pre-read or an app-layer authorization check. A wrong
    /// namespace, an ingested source (`adv_wisdom` / `adv_reflection`), an
    /// unknown id, or a repeated delete all return `0`.
    pub async fn forget_manual(&self, id: &str, namespace: &str) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM memories WHERE id = $1 AND namespace = $2 AND source = 'manual'",
        )
        .bind(id)
        .bind(namespace)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// AC3: the pool options carry the pinned, explicit lifecycle bounds. Pure
    /// and database-free — reads the configured values straight off the builder.
    #[test]
    fn pool_options_pin_lifecycle_bounds() {
        let opts = pool_options(7);
        assert_eq!(opts.get_max_connections(), 7);
        assert_eq!(opts.get_acquire_timeout(), Duration::from_secs(5));
        assert_eq!(opts.get_idle_timeout(), Some(Duration::from_secs(10 * 60)));
        assert_eq!(opts.get_max_lifetime(), Some(Duration::from_secs(30 * 60)));
        assert!(opts.get_test_before_acquire());
    }

    #[test]
    fn pool_options_propagate_max_connections() {
        // The configured pool size flows through to max_connections unchanged.
        assert_eq!(pool_options(1).get_max_connections(), 1);
        assert_eq!(pool_options(20).get_max_connections(), 20);
    }
}
