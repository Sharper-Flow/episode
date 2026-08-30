use episode::store::Store;
use episode::types::{EMBEDDING_DIM, MemoryInput, MemorySource, RecallFilters};
use sqlx::Row;

fn vector() -> Vec<f32> {
    let mut value = vec![0.0; EMBEDDING_DIM];
    value[0] = 1.0;
    value
}

/// A Product scope means "this Product's memories plus the shared pool", not
/// "only rows tagged with this Product". The product filter matches rows
/// tagged with the Product OR rows carrying no product claim at all, while a
/// different Product's tag — and an explicit `product: null` — stays excluded.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn product_scope_includes_untagged_shared_pool() {
    let db = std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string());
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_product_{}", uuid::Uuid::new_v4().simple());
    let shared = format!("it_shared_{}", uuid::Uuid::new_v4().simple());

    let outcome: anyhow::Result<(Vec<String>, Vec<String>)> = async {
        let seed = [
            (
                ns.clone(),
                "untagged-project",
                serde_json::json!({"work_id":"w"}),
            ),
            (
                shared.clone(),
                "untagged-shared",
                serde_json::json!({"tags":["b"]}),
            ),
            (
                ns.clone(),
                "tagged-p",
                serde_json::json!({"product":"p","work_id":"w"}),
            ),
            (
                shared.clone(),
                "tagged-q",
                serde_json::json!({"product":"q","work_id":"w"}),
            ),
            (
                ns.clone(),
                "null-product",
                serde_json::json!({"product":null,"work_id":"w"}),
            ),
        ];
        for (namespace, content, metadata) in seed {
            store
                .upsert(
                    &MemoryInput {
                        id: format!("{namespace}-{content}"),
                        namespace: namespace.clone(),
                        source: MemorySource::Manual,
                        source_id: None,
                        kind: Some("gotcha".into()),
                        content: content.into(),
                        metadata,
                    },
                    &vector(),
                )
                .await?;
        }

        let filters = RecallFilters {
            product: Some("p".into()),
            ..Default::default()
        };
        let mut product_hits: Vec<String> = store
            .recall(&vector(), &[ns.clone(), shared.clone()], 20, Some(&filters))
            .await?
            .into_iter()
            .map(|hit| hit.content)
            .collect();
        product_hits.sort();

        let composed = RecallFilters {
            product: Some("p".into()),
            work_id: Some("w".into()),
            kinds: Some(vec!["gotcha".into()]),
            ..Default::default()
        };
        let mut composed_hits: Vec<String> = store
            .recall(
                &vector(),
                &[ns.clone(), shared.clone()],
                20,
                Some(&composed),
            )
            .await?
            .into_iter()
            .map(|hit| hit.content)
            .collect();
        composed_hits.sort();

        Ok((product_hits, composed_hits))
    }
    .await;

    for namespace in [&ns, &shared] {
        sqlx::query("DELETE FROM memories WHERE namespace = $1")
            .bind(namespace)
            .execute(&pool)
            .await
            .expect("cleanup");
    }

    let (product_hits, composed_hits) = outcome.expect("seed and product-scoped recalls");
    assert_eq!(
        product_hits,
        vec![
            "tagged-p".to_string(),
            "untagged-project".to_string(),
            "untagged-shared".to_string(),
        ],
        "a Product scope returns the Product's rows plus the untagged shared pool"
    );
    assert_eq!(
        composed_hits,
        vec!["tagged-p".to_string(), "untagged-project".to_string()],
        "the Product-OR-unscoped arm composes with work and kind filters"
    );
}

#[tokio::test]
#[ignore = "requires Postgres"]
async fn recall_filters_compose_and_use_metadata_gin() {
    let db = std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string());
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_filters_{}", uuid::Uuid::new_v4().simple());
    let rows = [
        (
            "a",
            "gotcha",
            serde_json::json!({"product":"p","work_id":"w","tags":["a","b"]}),
        ),
        (
            "b",
            "decision",
            serde_json::json!({"product":"p","work_id":"w2","tags":["a"],"action":{"kind":"ad_hoc_resolved","summary":"done"}}),
        ),
        (
            "c",
            "gotcha",
            serde_json::json!({"product":"p","work_id":"w","tags":["a","b"],"action":{"kind":"open_followup"}}),
        ),
        (
            "d",
            "gotcha",
            serde_json::json!({"product":"q","tags":["b"]}),
        ),
    ];
    for (suffix, kind, metadata) in rows {
        store
            .upsert(
                &MemoryInput {
                    id: format!("{ns}-{suffix}"),
                    namespace: ns.clone(),
                    source: MemorySource::Manual,
                    source_id: None,
                    kind: Some(kind.into()),
                    content: suffix.into(),
                    metadata,
                },
                &vector(),
            )
            .await
            .expect("seed");
    }

    let matching = RecallFilters {
        product: Some("p".into()),
        work_id: Some("w".into()),
        tags: Some(vec!["a".into(), "b".into()]),
        kinds: Some(vec!["gotcha".into()]),
        sources: Some(vec![MemorySource::Manual]),
        max_age_days: Some(30),
        include_open_followups: false,
        include_promoted: false,
    };
    let hits = store
        .recall(&vector(), std::slice::from_ref(&ns), 20, Some(&matching))
        .await
        .expect("filtered");
    assert_eq!(
        hits.iter().map(|h| h.content.as_str()).collect::<Vec<_>>(),
        vec!["a"]
    );

    let including = RecallFilters {
        include_open_followups: true,
        ..matching.clone()
    };
    let mut ids = store
        .recall(&vector(), std::slice::from_ref(&ns), 20, Some(&including))
        .await
        .expect("include open")
        .into_iter()
        .map(|h| h.content)
        .collect::<Vec<_>>();
    ids.sort();
    assert_eq!(ids, vec!["a", "c"]);

    let any_kind = RecallFilters {
        product: Some("p".into()),
        kinds: Some(vec!["decision".into(), "gotcha".into()]),
        ..Default::default()
    };
    let mut ids = store
        .recall(&vector(), std::slice::from_ref(&ns), 20, Some(&any_kind))
        .await
        .expect("any kind")
        .into_iter()
        .map(|h| h.content)
        .collect::<Vec<_>>();
    ids.sort();
    assert_eq!(ids, vec!["a", "b"]);

    let index_definition: String = sqlx::query_scalar(
        "SELECT indexdef FROM pg_indexes WHERE indexname = 'memories_metadata_gin'",
    )
    .fetch_one(&pool)
    .await
    .expect("index catalog");
    assert!(index_definition.contains("metadata jsonb_path_ops"));
    let mut transaction = pool.begin().await.expect("transaction");
    sqlx::query("SET LOCAL enable_seqscan = off")
        .execute(&mut *transaction)
        .await
        .expect("disable seqscan");
    let plan = sqlx::query("EXPLAIN SELECT id FROM memories WHERE metadata @> $1")
        .bind(serde_json::json!({"product":"p"}))
        .fetch_all(&mut *transaction)
        .await
        .expect("explain")
        .into_iter()
        .map(|row| row.get::<String, _>(0))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(plan.contains("memories_metadata_gin"), "{plan}");
    transaction.rollback().await.expect("reset planner state");

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// Provenance and recency narrow recall on the typed columns: `sources`
/// filters the source column, `max_age_days` excludes rows whose first-capture
/// time is older than the cutoff. Both are pure filters — ranking is
/// untouched. Seed uses direct `created_at` backdating because the store's
/// upsert always stamps `now()`.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn sources_and_max_age_filter_recall() {
    let db = std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string());
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_prov_{}", uuid::Uuid::new_v4().simple());

    let seed = [
        (MemorySource::Manual, "manual-fresh"),
        (MemorySource::AdvWisdom, "wisdom-fresh"),
        (MemorySource::AdvReflection, "reflection-fresh"),
        (MemorySource::AdvWisdom, "wisdom-old"),
        (MemorySource::Manual, "manual-old"),
    ];
    for (source, content) in seed {
        store
            .upsert(
                &MemoryInput {
                    id: format!("{ns}-{content}"),
                    namespace: ns.clone(),
                    source,
                    source_id: (source != MemorySource::Manual)
                        .then(|| format!("{ns}-src-{content}")),
                    kind: Some("gotcha".into()),
                    content: content.into(),
                    metadata: serde_json::json!({}),
                },
                &vector(),
            )
            .await
            .expect("seed");
    }
    // Backdate the two `-old` rows 100 days; `upsert` stamps now().
    sqlx::query(
        "UPDATE memories SET created_at = now() - interval '100 days' \
                 WHERE namespace = $1 AND id LIKE '%-old'",
    )
    .bind(&ns)
    .execute(&pool)
    .await
    .expect("backdate");

    async fn hits(store: &Store, ns: &str, filters: Option<&RecallFilters>) -> Vec<String> {
        let namespaces = [ns.to_string()];
        store
            .recall(&vector(), &namespaces, 20, filters)
            .await
            .expect("recall")
            .into_iter()
            .map(|h| h.content)
            .collect()
    }

    let out_of_range = RecallFilters {
        max_age_days: Some(u32::MAX),
        ..Default::default()
    };
    assert!(
        store
            .recall(
                &vector(),
                std::slice::from_ref(&ns),
                20,
                Some(&out_of_range),
            )
            .await
            .is_err(),
        "the public store boundary must reject day counts that cannot bind as int4"
    );

    // Cleanup runs before assertions can panic.
    let result: anyhow::Result<(Vec<String>, Vec<String>, Vec<String>)> = async {
        let a = hits(
            &store,
            &ns,
            Some(&RecallFilters {
                sources: Some(vec![MemorySource::AdvWisdom]),
                ..Default::default()
            }),
        )
        .await;
        let b = hits(
            &store,
            &ns,
            Some(&RecallFilters {
                sources: Some(vec![MemorySource::Manual, MemorySource::AdvReflection]),
                ..Default::default()
            }),
        )
        .await;
        let c = hits(
            &store,
            &ns,
            Some(&RecallFilters {
                max_age_days: Some(30),
                ..Default::default()
            }),
        )
        .await;
        Ok((a, b, c))
    }
    .await;
    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");

    let (mut by_source, mut composed_sources, mut fresh_only) = result.expect("filtered recalls");
    by_source.sort();
    composed_sources.sort();
    fresh_only.sort();
    assert_eq!(
        by_source,
        vec!["wisdom-fresh".to_string(), "wisdom-old".to_string()],
        "sources filters the typed source column"
    );
    assert_eq!(
        composed_sources,
        vec![
            "manual-fresh".to_string(),
            "manual-old".to_string(),
            "reflection-fresh".to_string()
        ],
        "multiple sources union within the filter"
    );
    assert_eq!(
        fresh_only,
        vec![
            "manual-fresh".to_string(),
            "reflection-fresh".to_string(),
            "wisdom-fresh".to_string(),
        ],
        "max_age_days excludes rows first captured before the cutoff"
    );
}
