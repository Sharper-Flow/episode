//! Postgres + pgvector persistence.

use std::collections::HashSet;
use std::time::Duration;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::types::{
    MemoryInput, NamespaceStat, PROMOTION_STATE_KEY, PromotionState, PromotionStateKind,
    RecallFilters, RecallHit,
};

fn push_and(builder: &mut QueryBuilder<Postgres>, has_where: &mut bool) {
    if *has_where {
        builder.push(" AND ");
    } else {
        builder.push(" WHERE ");
        *has_where = true;
    }
}

fn build_recall_query(
    vector: pgvector::Vector,
    namespaces: &[String],
    top_k: i64,
    filters: Option<&RecallFilters>,
) -> QueryBuilder<Postgres> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT id, namespace, source, kind, content, metadata, (1.0 - (embedding <=> ",
    );
    builder
        .push_bind(vector.clone())
        .push("))::float8 AS score FROM memories");
    let mut has_where = false;
    if !namespaces.is_empty() {
        push_and(&mut builder, &mut has_where);
        builder
            .push("namespace = ANY(")
            .push_bind(namespaces.to_vec())
            .push(")");
    }
    if let Some(filters) = filters {
        let mut metadata = serde_json::Map::new();
        if let Some(product) = filters.product.as_ref() {
            metadata.insert("product".into(), product.clone().into());
        }
        if let Some(work_id) = filters.work_id.as_ref() {
            metadata.insert("work_id".into(), work_id.clone().into());
        }
        if let Some(tags) = filters.tags.as_ref() {
            metadata.insert("tags".into(), serde_json::json!(tags));
        }
        if !metadata.is_empty() {
            push_and(&mut builder, &mut has_where);
            builder
                .push("metadata @> ")
                .push_bind(serde_json::Value::Object(metadata));
        }
        if let Some(kinds) = filters.kinds.as_ref() {
            push_and(&mut builder, &mut has_where);
            builder
                .push("kind = ANY(")
                .push_bind(kinds.clone())
                .push(")");
        }
    }
    if !filters.is_some_and(|value| value.include_open_followups) {
        push_and(&mut builder, &mut has_where);
        builder
            .push("NOT (metadata @> ")
            .push_bind(serde_json::json!({"action":{"kind":"open_followup"}}))
            .push(")");
    }
    // Promoted rows are owned by a durable Concord record; serving episode's
    // copy alongside it lets the two disagree with no signal about which is law.
    //
    // This predicate is negative and therefore does NOT use the
    // `memories_metadata_gin` index, exactly as the open-followup exclusion
    // above does not. See `docs/specs/0011-promotion-state.md`.
    if !filters.is_some_and(|value| value.include_promoted) {
        push_and(&mut builder, &mut has_where);
        builder
            .push("NOT (metadata @> ")
            .push_bind(serde_json::json!({
                PROMOTION_STATE_KEY: {"kind": "promoted"}
            }))
            .push(")");
    }
    builder
        .push(" ORDER BY embedding <=> ")
        .push_bind(vector)
        .push(" LIMIT ")
        .push_bind(top_k);
    builder
}

/// Compare-and-set of a memory's promotion state.
///
/// Every state value is a bound parameter, so no state literal reaches SQL
/// syntax. The precondition uses `IS NOT DISTINCT FROM` rather than `=` so a
/// NULL parameter — meaning "no promotion state" — matches an absent key. That
/// is what lets one statement cover every transition, including the episodic
/// start state, without branching on the expected kind.
const PROMOTE_SQL: &str = "UPDATE memories \
     SET metadata = jsonb_set(metadata, ARRAY[$3], $4::jsonb, true), \
         updated_at = now() \
     WHERE id = $1 AND namespace = $2 \
       AND metadata -> $3 ->> 'kind' IS NOT DISTINCT FROM $5";

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

    /// Idempotent upsert keyed on stable `id`. Returns `Ok(())` on success.
    ///
    /// Delegates to [`Self::upsert_batch`] so the single-row and batch persistence
    /// paths share one SQL statement and transactional semantics. The acknowledged
    /// overhead of a one-row transaction is bounded and keeps the codebase free of
    /// duplicated `ON CONFLICT` clauses.
    pub async fn upsert(&self, input: &MemoryInput, embedding: &[f32]) -> Result<()> {
        self.upsert_batch(std::slice::from_ref(input), &[embedding.to_vec()])
            .await?;
        Ok(())
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
        filters: Option<&RecallFilters>,
    ) -> Result<Vec<RecallHit>> {
        let q = pgvector::Vector::from(query_embedding.to_vec());
        let mut query = build_recall_query(q, namespaces, top_k, filters);
        let rows = query.build().fetch_all(&self.pool).await?;

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

    /// Compare-and-set a memory's promotion state (AC5).
    ///
    /// Borrows [`Self::forget_manual`]'s shape — one statement, structural
    /// predicates, rows-affected return — but deliberately **not** its
    /// `source = 'manual'` restriction. The row a human most needs to promote is
    /// an ingested wisdom entry, so a manual-only predicate would forbid the
    /// primary use case.
    ///
    /// `from` is the expected current state. A mismatch, a wrong namespace, or
    /// an unknown id all return `0` rather than erroring.
    ///
    /// Safe under READ COMMITTED because the merge stays in SQL: a concurrent
    /// loser blocks on the row lock, re-evaluates the precondition against the
    /// winner's committed row, and matches nothing. Reading the state into Rust
    /// and writing it back would reintroduce the lost-update window.
    pub async fn promote(
        &self,
        id: &str,
        namespace: &str,
        to: &PromotionState,
        from: PromotionStateKind,
    ) -> Result<u64> {
        to.validate()?;
        let result = sqlx::query(PROMOTE_SQL)
            .bind(id)
            .bind(namespace)
            .bind(PROMOTION_STATE_KEY)
            .bind(serde_json::to_value(to)?)
            .bind(from.as_tag())
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
    use crate::types::EMBEDDING_DIM;

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

    #[test]
    fn recall_query_uses_bound_filters_and_default_open_exclusion() {
        let filters = RecallFilters {
            product: Some("secret-product".into()),
            work_id: Some("secret-work".into()),
            tags: Some(vec!["secret-tag".into()]),
            kinds: Some(vec!["secret-kind".into()]),
            include_open_followups: false,
            include_promoted: false,
        };
        let query = build_recall_query(
            pgvector::Vector::from(vec![1.0; EMBEDDING_DIM]),
            &["project".into(), "global".into()],
            8,
            Some(&filters),
        );
        let sql_text = query.sql();
        let sql = sql_text.as_str();
        assert!(sql.contains("namespace = ANY("));
        assert!(sql.contains("metadata @>"));
        assert!(sql.contains("kind = ANY("));
        assert!(sql.contains("NOT (metadata @>"));
        for value in ["secret-product", "secret-work", "secret-tag", "secret-kind"] {
            assert!(!sql.contains(value));
        }
    }

    fn promotion_query(filters: &RecallFilters) -> QueryBuilder<Postgres> {
        build_recall_query(
            pgvector::Vector::from(vec![1.0; EMBEDDING_DIM]),
            &["project".into()],
            8,
            Some(filters),
        )
    }

    /// AC3: the promoted exclusion is on by default, alongside the open-followup
    /// exclusion. Two independent `NOT (metadata @> $n)` clauses.
    #[test]
    fn recall_query_excludes_promoted_by_default() {
        let query = promotion_query(&RecallFilters::default());
        let sql_text = query.sql();
        let sql = sql_text.as_str();
        assert_eq!(
            sql.matches("NOT (metadata @>").count(),
            2,
            "default recall must exclude both open followups and promoted rows:\n{sql}"
        );
    }

    /// AC4: opting in drops only the promoted exclusion; the open-followup
    /// exclusion is independent and must survive.
    #[test]
    fn recall_query_omits_promoted_exclusion_when_opted_in() {
        let query = promotion_query(&RecallFilters {
            include_promoted: true,
            ..Default::default()
        });
        let sql_text = query.sql();
        let sql = sql_text.as_str();
        assert_eq!(
            sql.matches("NOT (metadata @>").count(),
            1,
            "opting into promoted rows must leave the open-followup exclusion:\n{sql}"
        );
    }

    /// Mirrors `recall_query_uses_bound_filters_and_default_open_exclusion`:
    /// promotion state literals are bound parameters, never SQL syntax.
    /// AC5: the transition statement carries no state literal and no source
    /// restriction, and asserts its precondition with NULL-safe comparison so a
    /// single statement covers the episodic start state.
    #[test]
    fn promote_sql_binds_state_and_omits_source_restriction() {
        for value in ["promotion_candidate", "promoted", "episodic"] {
            assert!(
                !PROMOTE_SQL.contains(value),
                "{value:?} must be a bound value, not SQL text:\n{PROMOTE_SQL}"
            );
        }
        assert!(
            !PROMOTE_SQL.contains("source"),
            "promote must not inherit forget_manual's source restriction; \
             the primary use case is promoting an ingested row:\n{PROMOTE_SQL}"
        );
        assert!(
            PROMOTE_SQL.contains("IS NOT DISTINCT FROM"),
            "the precondition must be NULL-safe so an absent key is matchable:\n{PROMOTE_SQL}"
        );
    }

    /// The episodic sentinel is the absence of a tag, which becomes SQL NULL.
    #[test]
    fn promotion_state_kind_tags_round_trip() {
        assert_eq!(PromotionStateKind::Episodic.as_tag(), None);
        assert_eq!(
            PromotionStateKind::PromotionCandidate.as_tag(),
            Some("promotion_candidate")
        );
        assert_eq!(PromotionStateKind::Promoted.as_tag(), Some("promoted"));
        assert_eq!(
            PromotionState::PromotionCandidate {}.kind(),
            PromotionStateKind::PromotionCandidate
        );
        assert_eq!(
            PromotionState::Promoted { target: "t".into() }.kind(),
            PromotionStateKind::Promoted
        );
    }

    #[test]
    fn recall_query_binds_promotion_state_rather_than_inlining_it() {
        let query = promotion_query(&RecallFilters::default());
        let sql_text = query.sql();
        let sql = sql_text.as_str();
        for value in ["promotion_state", "promoted"] {
            assert!(
                !sql.contains(value),
                "{value:?} must be a bound value, not SQL text:\n{sql}"
            );
        }
    }
}
