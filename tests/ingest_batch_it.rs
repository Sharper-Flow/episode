//! Batched ingestion + transactional persistence integration tests (AC4 / AC7,
//! design §4).
//!
//! Exercise the new bulk `Store` path against a real Postgres + pgvector:
//!   - bulk source-id dedup, including exploded reflection child ids;
//!   - 65 eligible items persisted across two bounded partitions (64 + 1);
//!   - transactional all-or-nothing: a failing partition rolls back entirely and
//!     does not poison a later partition (failed-partition isolation).
//!
//! The loop-level partition count (65 -> 2 batches) and dedup filter are covered
//! by DB-free unit tests in `src/lib.rs`; these tests prove the persistence
//! contract those helpers feed into.
//!
//! Isolation note: the `memories.id` primary key is global (not namespaced), so
//! every test id is prefixed with its unique namespace (`{ns}::{raw}`). This
//! keeps ids globally unique across concurrently running tests and against any
//! pre-existing rows in the shared dev database, while still proving that the
//! stable id — including the `:`-bearing reflection child form — round-trips
//! verbatim.
//!
//! Requires the dev Postgres (default `localhost:5434`). Ignored by default:
//!
//!   cargo test --test ingest_batch_it -- --ignored --nocapture
//!
//! Override the DB via `EPISODE_TEST_DATABASE_URL`.

use episode::store::Store;
use episode::types::{EMBEDDING_DIM, MemoryInput, MemorySource};

fn db_url() -> String {
    std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string())
}

/// Namespace-qualify a raw id so it is globally unique under the global `id`
/// primary key (see module-level isolation note).
fn nid(ns: &str, raw: &str) -> String {
    format!("{ns}::{raw}")
}

/// An ingested ADV item whose `source_id` equals its stable, namespace-qualified
/// `id` (the common case for wisdom entries and exploded reflection children).
fn adv(raw: &str, ns: &str) -> MemoryInput {
    let id = nid(ns, raw);
    MemoryInput {
        id: id.clone(),
        namespace: ns.to_string(),
        source: MemorySource::AdvWisdom,
        source_id: Some(id),
        kind: Some("gotcha".into()),
        content: format!("content for {raw}"),
        metadata: serde_json::json!({ "raw": raw }),
    }
}

fn good_vec() -> Vec<f32> {
    vec![0.0f32; EMBEDDING_DIM]
}

async fn ns_count(store: &Store, ns: &str) -> i64 {
    store
        .stats()
        .await
        .expect("stats")
        .into_iter()
        .filter(|s| s.namespace == ns)
        .map(|s| s.count)
        .sum()
}

async fn cleanup(db: &str, ns: &str) {
    // Test-only raw delete of the unique namespace; not the production deletion
    // path. Best-effort: a unique namespace cannot collide with future runs even
    // if this fails. Opens its own pool (mirrors `recall_it.rs`) so no test-only
    // surface is added to `Store`.
    if let Ok(pool) = sqlx::PgPool::connect(db).await {
        let _ = sqlx::query("DELETE FROM memories WHERE namespace = $1")
            .bind(ns)
            .execute(&pool)
            .await;
    }
}

/// AC4 / design §4: a single bulk lookup returns exactly the already-present
/// source ids, and exploded reflection child ids (`rf:*:kind:index`) round-trip
/// through the batch path unchanged.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn bulk_dedup_returns_existing_subset_and_preserves_reflection_ids() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let ns = format!("it_batch_{}", uuid::Uuid::new_v4().simple());

    let rf_child = nid(&ns, "rf-xyz:friction:0"); // exploded reflection child
    let items = vec![
        adv("pw-1", &ns),
        adv("pw-2", &ns),
        adv("rf-xyz:friction:0", &ns),
    ];
    let embs = vec![good_vec(); items.len()];
    let n = store.upsert_batch(&items, &embs).await.expect("seed batch");
    assert_eq!(n, 3, "three fresh items insert three rows");

    // All three are reported present in one bulk lookup.
    let all = store
        .existing_source_ids(
            &ns,
            &[
                nid(&ns, "pw-1"),
                nid(&ns, "pw-2"),
                nid(&ns, "rf-xyz:friction:0"),
            ],
        )
        .await
        .expect("existing all");
    assert_eq!(all.len(), 3);
    assert!(
        all.contains(&rf_child),
        "reflection child id must round-trip intact"
    );

    // A mixed candidate set returns only the existing subset; the unknown
    // `pw-new` is excluded and the reflection child is preserved.
    let pw_new = nid(&ns, "pw-new");
    let subset = store
        .existing_source_ids(
            &ns,
            &[
                nid(&ns, "pw-1"),
                nid(&ns, "rf-xyz:friction:0"),
                pw_new.clone(),
            ],
        )
        .await
        .expect("existing subset");
    assert_eq!(subset.len(), 2, "only the two existing ids are returned");
    assert!(subset.contains(&nid(&ns, "pw-1")));
    assert!(subset.contains(&rf_child));
    assert!(!subset.contains(&pw_new));

    // Empty candidate list short-circuits to an empty set.
    let none = store
        .existing_source_ids(&ns, &[])
        .await
        .expect("existing empty");
    assert!(none.is_empty());

    cleanup(&db, &ns).await;
}

/// AC4: 65 eligible items are persisted across two bounded partitions (64 + 1)
/// rather than one unbounded call, and a subsequent bulk lookup sees all 65.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn batch_upsert_65_via_two_partitions_persists_all() {
    use episode::scheduler::INGEST_BATCH_MAX;

    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let ns = format!("it_batch65_{}", uuid::Uuid::new_v4().simple());

    let raws: Vec<String> = (0..65).map(|i| format!("pw-{i}")).collect();
    let items: Vec<MemoryInput> = raws.iter().map(|r| adv(r, &ns)).collect();
    let embs: Vec<Vec<f32>> = (0..65).map(|_| good_vec()).collect();

    // Mirror the loop: partition into chunks of at most INGEST_BATCH_MAX.
    let mut total = 0usize;
    let mut partitions = 0usize;
    for (items_chunk, embs_chunk) in items
        .chunks(INGEST_BATCH_MAX)
        .zip(embs.chunks(INGEST_BATCH_MAX))
    {
        total += store
            .upsert_batch(items_chunk, embs_chunk)
            .await
            .expect("partition upsert");
        partitions += 1;
    }
    assert_eq!(
        partitions, 2,
        "65 items must become two partitions (64 + 1)"
    );
    assert_eq!(total, 65, "all 65 rows inserted");

    assert_eq!(ns_count(&store, &ns).await, 65);

    let all_ids: Vec<String> = raws.iter().map(|r| nid(&ns, r)).collect();
    let existing = store
        .existing_source_ids(&ns, &all_ids)
        .await
        .expect("existing 65");
    assert_eq!(
        existing.len(),
        65,
        "every id is present after two partitions"
    );

    cleanup(&db, &ns).await;
}

/// Design §4 / DONT3: a failing partition rolls back atomically (none of its
/// rows persist) and does not prevent a later partition from succeeding. A
/// wrong-dimension vector forces a server-side `vector(1024)` error mid-batch,
/// exercising the transaction rollback path that production normally avoids via
/// `validate_batch_output`.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn failed_partition_rolls_back_and_does_not_poison_next() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let ns = format!("it_batch_tx_{}", uuid::Uuid::new_v4().simple());

    // Partition A: three items, but the middle embedding has the wrong
    // dimensionality. The DB rejects the multi-row INSERT and the transaction
    // rolls back, so NONE of A's rows may persist — including the two good ones.
    let a_items = vec![adv("a-1", &ns), adv("a-bad", &ns), adv("a-3", &ns)];
    let a_embs = vec![
        good_vec(),
        vec![0.0f32; 7], // wrong dim -> server rejects vector(1024)
        good_vec(),
    ];
    let a_err = store
        .upsert_batch(&a_items, &a_embs)
        .await
        .expect_err("wrong-dimension batch must fail");
    let msg = a_err.to_string().to_ascii_lowercase();
    assert!(
        msg.contains("dimension") || msg.contains("expected") || msg.contains("1024"),
        "error should describe the dimension mismatch, got: {a_err}"
    );

    // Atomicity: not even the good rows of partition A survived.
    let a_existing = store
        .existing_source_ids(&ns, &[nid(&ns, "a-1"), nid(&ns, "a-bad"), nid(&ns, "a-3")])
        .await
        .expect("existing after rollback");
    assert!(
        a_existing.is_empty(),
        "a failed partition must roll back entirely, got: {a_existing:?}"
    );
    assert_eq!(ns_count(&store, &ns).await, 0);

    // Failed-partition isolation: a later, well-formed partition still persists.
    let b_items = vec![adv("b-1", &ns), adv("b-2", &ns)];
    let b_embs = vec![good_vec(), good_vec()];
    let b_n = store
        .upsert_batch(&b_items, &b_embs)
        .await
        .expect("later partition must succeed after a failed one");
    assert_eq!(b_n, 2);

    let b_existing = store
        .existing_source_ids(&ns, &[nid(&ns, "b-1"), nid(&ns, "b-2")])
        .await
        .expect("existing b");
    assert_eq!(b_existing.len(), 2, "later partition persisted fully");
    assert_eq!(ns_count(&store, &ns).await, 2);

    cleanup(&db, &ns).await;
}
