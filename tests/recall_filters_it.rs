use episode::store::Store;
use episode::types::{EMBEDDING_DIM, MemoryInput, MemorySource, RecallFilters};
use sqlx::Row;

fn vector() -> Vec<f32> {
    let mut value = vec![0.0; EMBEDDING_DIM];
    value[0] = 1.0;
    value
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
