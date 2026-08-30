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
            .await
            .expect("seed");
    }

    let filters = RecallFilters {
        product: Some("p".into()),
        ..Default::default()
    };
    let mut got: Vec<String> = store
        .recall(&vector(), &[ns.clone(), shared.clone()], 20, Some(&filters))
        .await
        .expect("product-scoped recall")
        .into_iter()
        .map(|h| h.content)
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec![
            "tagged-p".to_string(),
            "untagged-project".to_string(),
            "untagged-shared".to_string(),
        ],
        "a Product scope returns the Product's rows plus the untagged shared pool"
    );

    let composed = RecallFilters {
        product: Some("p".into()),
        work_id: Some("w".into()),
        kinds: Some(vec!["gotcha".into()]),
        ..Default::default()
    };
    let mut got: Vec<String> = store
        .recall(
            &vector(),
            &[ns.clone(), shared.clone()],
            20,
            Some(&composed),
        )
        .await
        .expect("product + work + kind recall")
        .into_iter()
        .map(|hit| hit.content)
        .collect();
    got.sort();
    assert_eq!(
        got,
        vec!["tagged-p".to_string(), "untagged-project".to_string()],
        "the Product-OR-unscoped arm composes with work and kind filters"
    );

    for namespace in [ns, shared] {
        sqlx::query("DELETE FROM memories WHERE namespace = $1")
            .bind(&namespace)
            .execute(&pool)
            .await
            .expect("cleanup");
    }
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
