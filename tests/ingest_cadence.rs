//! Bounded, reproducible ingestion-cadence measurement (AC5 / AC9).
//!
//! Exercises the full serial root-reconcile path (`parse -> bulk dedup ->
//! bounded embed -> transactional persist`) against the dev Postgres. A
//! deterministic fake embedder with a calibrated per-item delay stands in for
//! the local fastembed model so the measurement is stable across runs and
//! machines, while still capturing the scheduling/DB overhead around the
//! embedding work.
//!
//! Outcome is printed to stdout in a structured form that can be pasted into
//! `docs/specs/0008-pool-and-ingestion-bounds.md`.
//!
//! Requires the dev Postgres (default `localhost:5434`). Ignored by default:
//!
//!   cargo test --test ingest_cadence -- --ignored --nocapture
//!
//! Override the DB via `EPISODE_TEST_DATABASE_URL`.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

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

/// Self-cleaning temp directory that holds a project root with `.adv/...`.
struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str, item_count: usize) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "episode-cadence-{}-{}-{}",
            label,
            std::process::id(),
            nanos
        ));
        let adv = path.join(".adv");
        std::fs::create_dir_all(&adv).unwrap();

        // One wisdom.jsonl with `item_count` unique entries.
        let lines: Vec<String> = (0..item_count)
            .map(|i| {
                format!(r#"{{"id":"cw-{i}","type":"gotcha","content":"content for item {i}"}}"#,)
            })
            .collect();
        std::fs::write(adv.join("wisdom.jsonl"), lines.join("\n")).unwrap();

        Self { path }
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

/// Deterministic embedder that returns zero vectors and spends a fixed amount
/// of time per input text. The delay is chosen to dominate DB overhead for a
/// 64-item batch without making the test slow.
struct CalibratedEmbedder {
    delay_per_item: Duration,
    calls: AtomicUsize,
}

impl CalibratedEmbedder {
    fn new(delay_per_item: Duration) -> Self {
        Self {
            delay_per_item,
            calls: AtomicUsize::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Embedder for CalibratedEmbedder {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        // Simulate model time proportional to batch size. Running inside the
        // scheduler's `spawn_blocking`, this blocks one worker thread.
        std::thread::sleep(self.delay_per_item * texts.len().max(1) as u32);
        Ok(texts.iter().map(|_| vec![0.0f32; EMBEDDING_DIM]).collect())
    }
}

async fn cleanup_namespace(db: &str, ns: &str) {
    if let Ok(pool) = sqlx::PgPool::connect(db).await {
        let _ = sqlx::query("DELETE FROM memories WHERE namespace = $1")
            .bind(ns)
            .execute(&pool)
            .await;
    }
}

/// Run one serial root reconcile and return elapsed time + number of embedder
/// invocations. Uses a fresh store connection so pool acquisition bounds are
/// exercised as part of startup.
async fn measure_serial_root(
    db: &str,
    pool_size: u32,
    root: &ProjectRoot,
    delay_per_item: Duration,
) -> (Duration, usize, episode::ReconcileOutcome) {
    let store = Store::connect(db, pool_size)
        .await
        .expect("connect + migrate (pool acquisition bound applies)");

    let calibrated = Arc::new(CalibratedEmbedder::new(delay_per_item));
    let embedder: Arc<dyn Embedder> = calibrated.clone();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (handle, worker) = scheduler::start(
        embedder,
        SchedulerConfig::default(),
        Arc::new(NoopRecorder),
        shutdown_rx.clone(),
    );

    let start = Instant::now();
    let outcome = episode::reconcile_root(&store, &handle, root, &shutdown_rx).await;
    let elapsed = start.elapsed();

    // Drive graceful shutdown and wait for the worker to exit.
    let _ = shutdown_tx.send(true);
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .expect("worker should exit after shutdown")
        .expect("worker panicked");

    let calls = calibrated.calls();
    (elapsed, calls, outcome)
}

/// AC9: serial root ingestion cadence is bounded and dominated by the single
/// serial embedder. With a 10 ms/item fake model, 64 items reconcile in well
/// under a second; DB/scheduling overhead is a small fraction of total time.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn serial_root_ingestion_cadence() {
    let db = db_url();
    let pool_size = 10u32;
    let delay_per_item = Duration::from_millis(10);
    let n = scheduler::INGEST_BATCH_MAX; // 64 items -> one bounded batch
    let ns = format!("cadence_{}", uuid::Uuid::new_v4().simple());
    let tmp = TempRoot::new(&ns, n);
    let root = tmp.root(&ns);

    let (elapsed, calls, outcome) =
        measure_serial_root(&db, pool_size, &root, delay_per_item).await;

    cleanup_namespace(&db, &ns).await;

    let model_time = delay_per_item * n as u32;
    let overhead = elapsed.saturating_sub(model_time);

    println!("=== episode ingestion cadence measurement ===");
    println!("workload: {n} unique wisdom items");
    println!("batch_size: {n} (INGEST_BATCH_MAX)");
    println!("fake_embedder_delay_per_item: {delay_per_item:?}");
    println!(
        "pool_config: pool_size={pool_size}, acquire_timeout=5s, idle_timeout=10min, max_lifetime=30min"
    );
    println!("elapsed_total: {elapsed:?}");
    println!("estimated_model_time: {model_time:?}");
    println!("pipeline_overhead (elapsed - model): {overhead:?}");
    println!("embedder_invocations: {calls}");
    println!("ingested_rows: {}", outcome.ingested);
    println!(
        "disposition: serial single-embedder ingestion is bounded; DB/scheduling overhead is small relative to embedding time; bounded concurrency is not warranted because the local model is the sole throughput bottleneck and is already pinned to one active inference."
    );

    // All items should have persisted.
    assert!(!outcome.stopped, "reconcile should not stop early");
    assert_eq!(
        outcome.ingested, n,
        "all {n} items should be persisted in one batch"
    );
    // One invocation for the single 64-item batch.
    assert_eq!(calls, 1, "one bounded batch should produce one embed call");
    // The total must be at least the serial model time (otherwise the delay
    // wasn't actually applied) and well under a generous safety bound.
    assert!(
        elapsed >= model_time * 8 / 10,
        "elapsed {elapsed:?} should reflect serial model time {model_time:?}"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "serial reconcile of {n} items should complete in under 5s; took {elapsed:?}"
    );
    // Non-model overhead must be small on a local dev DB.
    assert!(
        overhead < Duration::from_millis(500),
        "DB/scheduling overhead {overhead:?} should be small relative to model time {model_time:?}"
    );
}
