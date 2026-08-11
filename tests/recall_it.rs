//! End-to-end integration: embed real text, persist, and semantically recall.
//!
//! Requires the dev Postgres (default `localhost:5434`) and downloads the
//! fastembed BGE-large model on first run. Ignored by default — run with:
//!
//!   cargo test --test recall_it -- --ignored --nocapture
//!
//! Override the DB via `EPISODE_TEST_DATABASE_URL`.

use episode::embed::{Embedder, LocalEmbedder};
use episode::store::Store;
use episode::types::{EMBEDDING_DIM, MemoryInput, MemorySource};

#[tokio::test]
#[ignore = "requires Postgres + downloads the embedding model"]
async fn embed_store_recall_ranks_semantically() {
    let db = std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string());

    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let embedder = LocalEmbedder::new().expect("init fastembed (downloads model on first run)");

    let ns = "it_test";
    let docs = [
        (
            "it-1",
            "Always run database migrations inside connect() on startup.",
        ),
        (
            "it-2",
            "The espresso machine needs descaling every two weeks.",
        ),
        (
            "it-3",
            "Use an HNSW index with cosine distance for vector similarity search.",
        ),
    ];

    // Clean slate for the test namespace.
    for (id, _) in docs {
        let _ = store.forget_manual(id, ns).await;
    }

    for (id, text) in docs {
        let emb = embedder.embed_one(text).expect("embed doc");
        let input = MemoryInput {
            id: id.to_string(),
            namespace: ns.to_string(),
            source: MemorySource::Manual,
            source_id: None,
            kind: Some("test".into()),
            content: text.to_string(),
            metadata: serde_json::json!({ "test": true }),
        };
        store.upsert(&input, &emb).await.expect("upsert");
    }

    // A query semantically closest to it-3 (vector index), unrelated to espresso.
    let q = embedder
        .embed_one("which index type is best for nearest-neighbour vector queries?")
        .expect("embed query");
    let hits = store
        .recall(&q, &[ns.to_string()], 3, None)
        .await
        .expect("recall");

    assert!(!hits.is_empty(), "recall returned no hits");
    for h in &hits {
        println!("  {:<6} score={:.4}  {}", h.id, h.score, h.content);
    }
    assert_eq!(
        hits[0].id, "it-3",
        "expected the vector-index memory to rank first"
    );
    assert!(hits[0].score > 0.0, "similarity score should be positive");

    // Cleanup.
    for (id, _) in docs {
        let _ = store.forget_manual(id, ns).await;
    }
}

/// AC1 / C3 / DONT1: `forget_manual` deletes only a matching `manual` row in the
/// given namespace, enforced by a single SQL predicate — no pre-read and no
/// app-layer source check. Wrong namespace, an ingested source, an unknown id,
/// and a repeated delete must all return `removed = 0`; only the exact
/// `(id, namespace, manual)` match returns `1`.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn forget_manual_enforces_namespace_and_manual_source() {
    let db = std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string());
    let store = Store::connect(&db, 5).await.expect("connect + migrate");

    // Unique namespace per run so this test never collides with other data.
    let ns = format!("it_delete_{}", uuid::Uuid::new_v4().simple());
    let wrong_ns = format!("{ns}_wrong");
    let zero_vec = vec![0.0f32; EMBEDDING_DIM];

    let manual_id = format!("{ns}-manual");
    let ingested_id = format!("{ns}-ingested");
    let ingested_source_id = format!("{ingested_id}-src");

    // Seed one manual row and one ingested (adv_wisdom) row in `ns`.
    let manual = MemoryInput {
        id: manual_id.clone(),
        namespace: ns.clone(),
        source: MemorySource::Manual,
        source_id: None,
        kind: Some("test".into()),
        content: "manual row that should be deletable".to_string(),
        metadata: serde_json::json!({ "test": true }),
    };
    let ingested = MemoryInput {
        id: ingested_id.clone(),
        namespace: ns.clone(),
        source: MemorySource::AdvWisdom,
        source_id: Some(ingested_source_id.clone()),
        kind: Some("gotcha".into()),
        content: "ingested row that must be protected from manual deletion".to_string(),
        metadata: serde_json::json!({ "test": true }),
    };
    store
        .upsert(&manual, &zero_vec)
        .await
        .expect("upsert manual");
    store
        .upsert(&ingested, &zero_vec)
        .await
        .expect("upsert ingested");

    // 1) Wrong namespace -> 0 (the manual row must survive untouched).
    assert_eq!(
        store
            .forget_manual(&manual_id, &wrong_ns)
            .await
            .expect("forget wrong namespace"),
        0,
        "wrong namespace must not delete the manual row"
    );

    // 2) Ingested source in the correct namespace -> 0 (protected).
    assert_eq!(
        store
            .forget_manual(&ingested_id, &ns)
            .await
            .expect("forget ingested"),
        0,
        "ingested source must be protected from manual deletion"
    );

    // 3) Unknown id -> 0.
    assert_eq!(
        store
            .forget_manual(&format!("{ns}-does-not-exist"), &ns)
            .await
            .expect("forget unknown id"),
        0,
        "unknown id must delete nothing"
    );

    // 4) Exact (id, namespace, manual) match -> 1.
    assert_eq!(
        store
            .forget_manual(&manual_id, &ns)
            .await
            .expect("forget manual match"),
        1,
        "matching manual row must be deleted exactly once"
    );

    // 5) Repeated delete -> 0 (already gone).
    assert_eq!(
        store
            .forget_manual(&manual_id, &ns)
            .await
            .expect("forget manual again"),
        0,
        "repeated delete must return 0"
    );

    // The ingested row must survive the entire manual-deletion sequence.
    assert!(
        store
            .exists_source(&ns, &ingested_source_id)
            .await
            .expect("exists_source"),
        "ingested row must survive manual-only deletion"
    );

    // Cleanup (test-only raw delete of the unique namespace; not the production
    // deletion path under test). Best-effort: unique namespace means leftovers
    // cannot collide with future runs even if this fails.
    if let Ok(pool) = sqlx::PgPool::connect(&db).await {
        let _ = sqlx::query("DELETE FROM memories WHERE namespace = $1")
            .bind(&ns)
            .execute(&pool)
            .await;
    }
}
