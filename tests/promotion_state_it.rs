//! Database-backed promotion state behavior.
//!
//! Recall exclusion is the episode half of Concord's promotion-receiving
//! contract: once a lesson graduates into a durable record, episode stops
//! serving its own copy so the two cannot disagree silently.

use episode::store::Store;
use episode::types::{
    EMBEDDING_DIM, MemoryInput, MemorySource, PROMOTION_STATE_KEY, PromotionState,
    PromotionStateKind, RecallFilters,
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

/// Ids are unique per namespace, not globally: two projects can both hold a
/// `pw-1`. Every row lookup in these tests binds the namespace beside the id.
async fn stored_state(pool: &sqlx::PgPool, ns: &str, id: &str) -> Option<PromotionState> {
    let raw: serde_json::Value =
        sqlx::query_scalar("SELECT metadata FROM memories WHERE id = $1 AND namespace = $2")
            .bind(id)
            .bind(ns)
            .fetch_one(pool)
            .await
            .expect("read metadata");
    raw.get(PROMOTION_STATE_KEY)
        .map(|value| serde_json::from_value(value.clone()).expect("stored state must be valid"))
}

/// AC5: transitions run in both directions. Demotion is the recovery path for a
/// `Promoted` row whose Concord target was deleted — without it that row would
/// be excluded from recall permanently with no way back.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn promote_transitions_run_in_both_directions() {
    let db = database_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_{}", uuid::Uuid::new_v4().simple());
    let id = format!("{ns}-a");

    seed(&store, &ns, "a", None).await;
    assert_eq!(
        stored_state(&pool, &ns, &id).await,
        None,
        "seeds start episodic"
    );

    // Episodic -> candidate.
    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::PromotionCandidate {},
                PromotionStateKind::Episodic
            )
            .await
            .expect("flag candidate"),
        1
    );
    assert_eq!(
        stored_state(&pool, &ns, &id).await,
        Some(PromotionState::PromotionCandidate {})
    );

    // Candidate -> promoted.
    let target = "docs/specs/0011-promotion-state.md#sha256:aaa";
    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::Promoted {
                    target: target.into()
                },
                PromotionStateKind::PromotionCandidate
            )
            .await
            .expect("graduate"),
        1
    );
    assert_eq!(
        stored_state(&pool, &ns, &id).await,
        Some(PromotionState::Promoted {
            target: target.into()
        })
    );

    // Promoted -> promoted, retargeted. The record version moved.
    let retarget = "docs/specs/0011-promotion-state.md#sha256:bbb";
    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::Promoted {
                    target: retarget.into()
                },
                PromotionStateKind::Promoted
            )
            .await
            .expect("retarget"),
        1
    );
    assert_eq!(
        stored_state(&pool, &ns, &id).await,
        Some(PromotionState::Promoted {
            target: retarget.into()
        })
    );

    // Promoted -> candidate. The Concord target died; the memory must resurface.
    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::PromotionCandidate {},
                PromotionStateKind::Promoted
            )
            .await
            .expect("demote"),
        1
    );
    assert_eq!(
        stored_state(&pool, &ns, &id).await,
        Some(PromotionState::PromotionCandidate {})
    );
    assert_eq!(
        recalled(&store, &ns, &RecallFilters::default()).await,
        vec!["a"],
        "a demoted row must be visible in default recall again"
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// AC5: the from-state precondition lives in SQL, so a mismatch affects zero
/// rows and leaves the stored state untouched.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn promote_rejects_from_state_mismatch() {
    let db = database_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_cas_{}", uuid::Uuid::new_v4().simple());
    let id = format!("{ns}-a");

    seed(
        &store,
        &ns,
        "a",
        Some(PromotionState::PromotionCandidate {}),
    )
    .await;

    // Claiming the row is still episodic must not win.
    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::Promoted {
                    target: "wrong".into()
                },
                PromotionStateKind::Episodic
            )
            .await
            .expect("mismatch is not an error"),
        0
    );
    // Claiming it is already promoted must not win either.
    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::Promoted {
                    target: "wrong".into()
                },
                PromotionStateKind::Promoted
            )
            .await
            .expect("mismatch is not an error"),
        0
    );
    assert_eq!(
        stored_state(&pool, &ns, &id).await,
        Some(PromotionState::PromotionCandidate {}),
        "a losing transition must leave the state untouched"
    );

    // Wrong namespace and unknown id are also no-ops.
    assert_eq!(
        store
            .promote(
                &id,
                "other-namespace",
                &PromotionState::PromotionCandidate {},
                PromotionStateKind::PromotionCandidate
            )
            .await
            .expect("wrong namespace"),
        0
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// AC1 / AC5: episodic means that the key is absent. A present but malformed
/// value must not satisfy the episodic compare-and-set precondition.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn promote_rejects_present_state_when_from_is_episodic() {
    let db = database_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_present_{}", uuid::Uuid::new_v4().simple());

    for (suffix, malformed) in [
        ("null", serde_json::Value::Null),
        ("object", serde_json::json!({})),
    ] {
        let id = format!("{ns}-{suffix}");
        store
            .upsert(
                &MemoryInput {
                    id: id.clone(),
                    namespace: ns.clone(),
                    source: MemorySource::Manual,
                    source_id: None,
                    kind: Some("gotcha".into()),
                    content: suffix.into(),
                    metadata: serde_json::json!({ PROMOTION_STATE_KEY: malformed }),
                },
                &vector(),
            )
            .await
            .expect("seed malformed state");

        assert_eq!(
            store
                .promote(
                    &id,
                    &ns,
                    &PromotionState::PromotionCandidate {},
                    PromotionStateKind::Episodic,
                )
                .await
                .expect("present state is a mismatch"),
            0,
            "a present promotion_state key is not episodic: {malformed}"
        );
    }

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// AC5: `promote` borrows `forget_manual`'s shape but NOT its `source = 'manual'`
/// restriction. The row a human most needs to promote is an ingested wisdom
/// entry, so a manual-only predicate would forbid the primary use case.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn promote_accepts_ingested_rows() {
    let db = database_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_ingested_{}", uuid::Uuid::new_v4().simple());
    let id = format!("{ns}-pw-1");

    store
        .upsert(
            &MemoryInput {
                id: id.clone(),
                namespace: ns.clone(),
                source: MemorySource::AdvWisdom,
                source_id: Some(format!("{ns}-src-1")),
                kind: Some("gotcha".into()),
                content: "ingested".into(),
                metadata: metadata_with(None),
            },
            &vector(),
        )
        .await
        .expect("seed ingested");

    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::Promoted {
                    target: "docs/specs/0011-promotion-state.md#sha256:ccc".into()
                },
                PromotionStateKind::Episodic
            )
            .await
            .expect("promote ingested row"),
        1,
        "an ingested row must be promotable; this is the primary use case"
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// A reconcile can decide that a row is absent, then lose its insert race to
/// another reconcile. A promotion between those writes must survive the losing
/// reconcile's `ON CONFLICT` path.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn concurrent_ingest_conflict_preserves_promotion_state() {
    let db = database_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_ingest_race_{}", uuid::Uuid::new_v4().simple());
    let id = format!("{ns}-pw-1");
    let input = MemoryInput {
        id: id.clone(),
        namespace: ns.clone(),
        source: MemorySource::AdvWisdom,
        source_id: Some(format!("{ns}-src-1")),
        kind: Some("gotcha".into()),
        content: "ingested".into(),
        metadata: metadata_with(None),
    };

    store.upsert(&input, &vector()).await.expect("first ingest");
    let target = "docs/specs/0011-promotion-state.md#sha256:race";
    assert_eq!(
        store
            .promote(
                &id,
                &ns,
                &PromotionState::Promoted {
                    target: target.into(),
                },
                PromotionStateKind::Episodic,
            )
            .await
            .expect("promote between ingest writes"),
        1
    );

    store
        .upsert(&input, &vector())
        .await
        .expect("losing ingest conflict");
    assert_eq!(
        stored_state(&pool, &ns, &id).await,
        Some(PromotionState::Promoted {
            target: target.into(),
        }),
        "an ingest conflict must not overwrite a concurrent promotion"
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}
