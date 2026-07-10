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
use episode::types::{MemoryInput, MemorySource};

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
        let _ = store.forget(id).await;
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
        assert!(store.upsert(&input, &emb).await.expect("upsert"));
    }

    // A query semantically closest to it-3 (vector index), unrelated to espresso.
    let q = embedder
        .embed_one("which index type is best for nearest-neighbour vector queries?")
        .expect("embed query");
    let hits = store
        .recall(&q, &[ns.to_string()], 3)
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
        let _ = store.forget(id).await;
    }
}
