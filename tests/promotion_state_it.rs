//! Database-backed promotion state behavior.
//!
//! Recall exclusion is the episode half of Concord's promotion-receiving
//! contract: once a lesson graduates into a durable record, episode stops
//! serving its own copy so the two cannot disagree silently.

use episode::store::Store;
use episode::types::{
    EMBEDDING_DIM, MemoryInput, MemorySource, PROMOTION_STATE_KEY, PromotionState, RecallFilters,
};

fn vector() -> Vec<f32> {
    let mut value = vec![0.0; EMBEDDING_DIM];
    value[0] = 1.0;
    value
}

fn database_url() -> String {
    std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string())
}

fn metadata_with(state: Option<PromotionState>) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("product".into(), "p".into());
    if let Some(state) = state {
        map.insert(
            PROMOTION_STATE_KEY.into(),
            serde_json::to_value(state).expect("promotion state serializes"),
        );
    }
    serde_json::Value::Object(map)
}

async fn seed(store: &Store, ns: &str, suffix: &str, state: Option<PromotionState>) {
    store
        .upsert(
            &MemoryInput {
                id: format!("{ns}-{suffix}"),
                namespace: ns.to_string(),
                source: MemorySource::Manual,
                source_id: None,
                kind: Some("gotcha".into()),
                content: suffix.into(),
                metadata: metadata_with(state),
            },
            &vector(),
        )
        .await
        .expect("seed");
}

async fn recalled(store: &Store, ns: &str, filters: &RecallFilters) -> Vec<String> {
    let mut found = store
        .recall(
            &vector(),
            std::slice::from_ref(&ns.to_string()),
            20,
            Some(filters),
        )
        .await
        .expect("recall")
        .into_iter()
        .map(|hit| hit.content)
        .collect::<Vec<_>>();
    found.sort();
    found
}

/// AC3 / AC4: promoted rows are excluded by default and returned on opt-in.
/// `PromotionCandidate` stays visible in both modes — it has not graduated, so
/// episode still holds the authoritative copy.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn recall_excludes_promoted_by_default_and_keeps_candidates_visible() {
    let db = database_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promotion_{}", uuid::Uuid::new_v4().simple());

    seed(&store, &ns, "plain", None).await;
    seed(
        &store,
        &ns,
        "candidate",
        Some(PromotionState::PromotionCandidate {}),
    )
    .await;
    seed(
        &store,
        &ns,
        "promoted",
        Some(PromotionState::Promoted {
            target: "docs/specs/0011-promotion-state.md#sha256:abc".into(),
        }),
    )
    .await;

    assert_eq!(
        recalled(&store, &ns, &RecallFilters::default()).await,
        vec!["candidate", "plain"],
        "default recall must hide promoted rows and keep candidates"
    );

    assert_eq!(
        recalled(
            &store,
            &ns,
            &RecallFilters {
                include_promoted: true,
                ..Default::default()
            }
        )
        .await,
        vec!["candidate", "plain", "promoted"],
        "opting in must return promoted rows alongside the rest"
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}
