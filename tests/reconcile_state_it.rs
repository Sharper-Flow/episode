//! Post-ingest state reconciliation against the dev Postgres.
//!
//! Episode's ingest is write-once per `source_id`: `filter_eligible` drops every
//! already-stored item, so `upsert_batch` only ever sees new rows. That makes
//! episode blind to *post-ingest* mutations of ADV state — and both fields this
//! change depends on are exactly that. `invalidated_by` is set when a lesson is
//! retracted, `promoted_at` when it graduates, and both happen long after first
//! ingest.
//!
//! These tests drive the real reconcile path twice over the same source file,
//! mutating ADV state in between. They fail against a parse-time-only design.
//!
//!   cargo test --test reconcile_state_it -- --ignored

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use episode::config::ProjectRoot;
use episode::embed::Embedder;
use episode::scheduler::{self, NoopRecorder, SchedulerConfig};
use episode::store::Store;
use episode::types::EMBEDDING_DIM;
use tokio::sync::watch;

fn db_url() -> String {
    std::env::var("EPISODE_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://episode:episode@localhost:5434/episode".to_string())
}

/// Zero-vector embedder. These tests assert persistence and state transitions,
/// never similarity, so embedding content is irrelevant.
struct ZeroEmbedder;

impl Embedder for ZeroEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.0f32; EMBEDDING_DIM]).collect())
    }
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "episode-reconcile-{}-{}-{}",
            label,
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(path.join(".adv")).unwrap();
        Self { path }
    }

    /// Rewrite `wisdom.jsonl`, standing in for ADV mutating its own state
    /// between reconciles.
    fn write_wisdom(&self, lines: &[&str]) {
        std::fs::write(
            self.path.join(".adv").join("wisdom.jsonl"),
            lines.join("\n"),
        )
        .unwrap();
    }

    fn root(&self, namespace: &str) -> ProjectRoot {
        ProjectRoot {
            namespace: namespace.to_string(),
            path: self.path.clone(),
        }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

async fn reconcile_once(store: &Store, root: &ProjectRoot) -> episode::ReconcileOutcome {
    let embedder: Arc<dyn Embedder> = Arc::new(ZeroEmbedder);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (handle, worker) = scheduler::start(
        embedder,
        SchedulerConfig::default(),
        Arc::new(NoopRecorder),
        shutdown_rx.clone(),
    );
    let outcome = episode::reconcile_root(store, &handle, root, &shutdown_rx).await;
    let _ = shutdown_tx.send(true);
    tokio::time::timeout(std::time::Duration::from_secs(5), worker)
        .await
        .expect("worker should exit after shutdown")
        .expect("worker panicked");
    outcome
}

async fn stored_ids(pool: &sqlx::PgPool, ns: &str) -> Vec<String> {
    let mut ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM memories WHERE namespace = $1 ORDER BY id")
            .bind(ns)
            .fetch_all(pool)
            .await
            .expect("read ids");
    ids.sort();
    ids
}

/// AC6: a wisdom entry ingested while valid, then retracted in ADV, must be
/// removed from the store.
///
/// This is the confirmed defect. The parse-time skip prevents *adding* an
/// invalidated entry but cannot remove one already stored, and the only DELETE
/// in the codebase is restricted to `source = 'manual'`, so ingested rows had no
/// removal path at all. The row kept surfacing in recall indefinitely.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn reconcile_removes_rows_retracted_after_ingest() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_retract_{}", uuid::Uuid::new_v4().simple());
    let tmp = TempRoot::new("retract");
    let root = tmp.root(&ns);

    // Pass 1: both entries are valid and get stored.
    tmp.write_wisdom(&[
        r#"{"id":"pw-keep","type":"gotcha","content":"still true"}"#,
        r#"{"id":"pw-drop","type":"gotcha","content":"retracted later"}"#,
    ]);
    assert_eq!(reconcile_once(&store, &root).await.ingested, 2);
    assert_eq!(
        stored_ids(&pool, &ns).await,
        vec!["pw-drop".to_string(), "pw-keep".to_string()]
    );

    // ADV retracts one entry. Dedup would otherwise skip the row entirely.
    tmp.write_wisdom(&[
        r#"{"id":"pw-keep","type":"gotcha","content":"still true"}"#,
        r#"{"id":"pw-drop","type":"gotcha","content":"retracted later","invalidated_by":"pw-9"}"#,
    ]);
    reconcile_once(&store, &root).await;
    assert_eq!(
        stored_ids(&pool, &ns).await,
        vec!["pw-keep".to_string()],
        "a retracted entry must not keep surfacing after it is already stored"
    );

    // Deletion is self-healing: un-invalidating restores the row, because an
    // absent row is absent from `existing_source_ids` and re-ingests fresh.
    tmp.write_wisdom(&[
        r#"{"id":"pw-keep","type":"gotcha","content":"still true"}"#,
        r#"{"id":"pw-drop","type":"gotcha","content":"retracted later"}"#,
    ]);
    reconcile_once(&store, &root).await;
    assert_eq!(
        stored_ids(&pool, &ns).await,
        vec!["pw-drop".to_string(), "pw-keep".to_string()],
        "un-invalidating in ADV must restore the memory"
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// Retraction is scoped to the namespace being reconciled.
///
/// Ids differ per namespace because `id` is a global primary key today, so
/// reusing one id across namespaces would exercise upsert collision rather than
/// delete scoping.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn retraction_is_scoped_to_the_reconciled_namespace() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_scope_a_{}", uuid::Uuid::new_v4().simple());
    let other = format!("it_scope_b_{}", uuid::Uuid::new_v4().simple());

    let tmp_a = TempRoot::new("scope-a");
    let root_a = tmp_a.root(&ns);
    tmp_a.write_wisdom(&[r#"{"id":"pw-a1","type":"gotcha","content":"namespace a"}"#]);
    reconcile_once(&store, &root_a).await;

    let tmp_b = TempRoot::new("scope-b");
    let root_b = tmp_b.root(&other);
    tmp_b.write_wisdom(&[r#"{"id":"pw-b1","type":"gotcha","content":"namespace b"}"#]);
    reconcile_once(&store, &root_b).await;

    // Retract only in namespace A.
    tmp_a.write_wisdom(&[
        r#"{"id":"pw-a1","type":"gotcha","content":"namespace a","invalidated_by":"pw-9"}"#,
    ]);
    reconcile_once(&store, &root_a).await;

    assert!(stored_ids(&pool, &ns).await.is_empty());
    assert_eq!(
        stored_ids(&pool, &other).await,
        vec!["pw-b1".to_string()],
        "retraction in one namespace must not delete another namespace's row"
    );

    for namespace in [&ns, &other] {
        sqlx::query("DELETE FROM memories WHERE namespace = $1")
            .bind(namespace)
            .execute(&pool)
            .await
            .expect("cleanup");
    }
}
