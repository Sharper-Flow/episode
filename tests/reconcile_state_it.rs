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
use episode::types::{
    EMBEDDING_DIM, PROMOTION_STATE_KEY, PromotionState, PromotionStateKind, RecallFilters,
};
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

/// Ids are unique per namespace, not globally: two projects can both hold a
/// `pw-1`. Every row lookup in these tests binds the namespace beside the id.
async fn stored_metadata(pool: &sqlx::PgPool, ns: &str, id: &str) -> serde_json::Value {
    sqlx::query_scalar("SELECT metadata FROM memories WHERE id = $1 AND namespace = $2")
        .bind(id)
        .bind(ns)
        .fetch_one(pool)
        .await
        .expect("read metadata")
}

async fn stored_state(pool: &sqlx::PgPool, ns: &str, id: &str) -> Option<PromotionState> {
    stored_metadata(pool, ns, id)
        .await
        .get(PROMOTION_STATE_KEY)
        .map(|value| serde_json::from_value(value.clone()).expect("valid stored state"))
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

/// AC6: a `promoted_at` set in ADV *after* first ingest must reach the stored
/// row.
///
/// This is the case that makes the mechanism worth having. Wisdom is
/// overwhelmingly ingested while still episodic and promoted later, so dedup
/// skips the row on every subsequent pass. A parse-time-only mapping would be
/// inert in the common case and the feature would appear to work while doing
/// nothing.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn reconcile_maps_promotion_recorded_after_ingest() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_late_{}", uuid::Uuid::new_v4().simple());
    let tmp = TempRoot::new("promote-late");
    let root = tmp.root(&ns);

    // Pass 1: ordinary episodic entry.
    tmp.write_wisdom(&[r#"{"id":"pw-late","type":"gotcha","content":"graduates later"}"#]);
    assert_eq!(reconcile_once(&store, &root).await.ingested, 1);
    assert_eq!(
        stored_state(&pool, &ns, "pw-late").await,
        None,
        "an unpromoted entry must stay episodic"
    );

    // ADV graduates it. Dedup skips the row, so only a direct update can apply.
    tmp.write_wisdom(&[
        r#"{"id":"pw-late","type":"gotcha","content":"graduates later","promoted_at":"2026-08-30T00:00:00Z"}"#,
    ]);
    reconcile_once(&store, &root).await;
    assert_eq!(
        stored_state(&pool, &ns, "pw-late").await,
        Some(PromotionState::PromotionCandidate {}),
        "promotion recorded after ingest must reach the stored row"
    );

    // A candidate has not graduated into a durable record yet, so episode still
    // holds the authoritative copy and must keep serving it.
    let hits = store
        .recall(
            &vec![0.0f32; EMBEDDING_DIM],
            std::slice::from_ref(&ns),
            10,
            Some(&RecallFilters::default()),
        )
        .await
        .expect("recall");
    assert_eq!(hits.len(), 1, "a candidate must stay visible in recall");

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// AC7: reconcile is idempotent and must never walk a human's promotion back.
///
/// `promoted_at` stays in the source file forever, so every later pass re-collects
/// the id. Without the absent-key guard each pass would reset a human-set
/// `Promoted { target }` to a candidate, silently un-graduating the memory and
/// resurrecting it in recall alongside the Concord record that owns it.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn reconcile_never_clobbers_a_human_promotion() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_idem_{}", uuid::Uuid::new_v4().simple());
    let tmp = TempRoot::new("promote-idem");
    let root = tmp.root(&ns);

    tmp.write_wisdom(&[
        r#"{"id":"pw-idem","type":"gotcha","content":"graduated","promoted_at":"2026-08-30T00:00:00Z"}"#,
    ]);
    reconcile_once(&store, &root).await;
    assert_eq!(
        stored_state(&pool, &ns, "pw-idem").await,
        Some(PromotionState::PromotionCandidate {}),
        "an entry promoted before first ingest must map too"
    );

    // A human supplies the Concord target that ADV cannot.
    let target = "docs/specs/0011-promotion-state.md#sha256:abc";
    assert_eq!(
        store
            .promote(
                "pw-idem",
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

    // Two further passes must both be no-ops against the promotion state.
    reconcile_once(&store, &root).await;
    reconcile_once(&store, &root).await;
    assert_eq!(
        stored_state(&pool, &ns, "pw-idem").await,
        Some(PromotionState::Promoted {
            target: target.into()
        }),
        "reconcile must not reset a human-set promotion to a candidate"
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// `promotion_state` is the single source of truth. Leaving the raw ADV
/// timestamp behind would give agents a second, differently-populated field to
/// filter on: present on entries promoted before first ingest, absent on
/// entries promoted after it, because dedup freezes metadata at first write.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn stored_metadata_drops_the_raw_promoted_at_field() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_promote_strip_{}", uuid::Uuid::new_v4().simple());
    let tmp = TempRoot::new("promote-strip");
    let root = tmp.root(&ns);

    tmp.write_wisdom(&[
        r#"{"id":"pw-strip","type":"gotcha","content":"graduated","promoted_at":"2026-08-30T00:00:00Z","source_change":"c1"}"#,
    ]);
    reconcile_once(&store, &root).await;

    let metadata = stored_metadata(&pool, &ns, "pw-strip").await;
    assert!(
        metadata.get("promoted_at").is_none(),
        "promoted_at must not survive alongside promotion_state: {metadata}"
    );
    assert_eq!(
        metadata.get("source_change").and_then(|v| v.as_str()),
        Some("c1"),
        "unrelated provenance must be preserved"
    );

    sqlx::query("DELETE FROM memories WHERE namespace = $1")
        .bind(&ns)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// `promotion_state` is episode's key, not the source file's.
///
/// `parse_wisdom` copies the raw ADV object into metadata, so every key an entry
/// carries lands in the store verbatim. Without an explicit strip, a wisdom
/// entry could set episode's reserved key directly — a well-formed `promoted`
/// value would hide that memory from recall the moment it was first ingested,
/// and a malformed value would put the row in a state no transition can address.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn source_supplied_promotion_state_cannot_hijack_the_reserved_key() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");
    let ns = format!("it_hijack_{}", uuid::Uuid::new_v4().simple());
    let tmp = TempRoot::new("hijack");
    let root = tmp.root(&ns);

    tmp.write_wisdom(&[
        r#"{"id":"pw-hijack","type":"gotcha","content":"still ours","promotion_state":{"kind":"promoted","target":"forged"},"source_change":"c1"}"#,
        r#"{"id":"pw-garbage","type":"gotcha","content":"malformed","promotion_state":"not-an-object"}"#,
    ]);
    reconcile_once(&store, &root).await;

    assert_eq!(
        stored_state(&pool, &ns, "pw-hijack").await,
        None,
        "a source-supplied promotion state must not reach the store"
    );
    assert_eq!(
        stored_state(&pool, &ns, "pw-garbage").await,
        None,
        "a malformed source-supplied value must not reach the store either"
    );
    assert_eq!(
        stored_metadata(&pool, &ns, "pw-hijack")
            .await
            .get("source_change")
            .and_then(|v| v.as_str()),
        Some("c1"),
        "stripping the reserved key must not disturb real provenance"
    );

    // Neither entry carries promoted_at, so both must stay visible in recall.
    let hits = store
        .recall(
            &vec![0.0f32; EMBEDDING_DIM],
            std::slice::from_ref(&ns),
            10,
            Some(&RecallFilters::default()),
        )
        .await
        .expect("recall");
    assert_eq!(
        hits.len(),
        2,
        "a forged promotion state must not hide a memory from recall"
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

/// Ingested ids are raw per-project ADV ids, so two watched projects both hold
/// a `pw-1`. A row's identity is (namespace, id): project B's reconcile must
/// not steal, move, or overwrite project A's row, and a promotion in B must
/// not touch A's same-id row.
#[tokio::test]
#[ignore = "requires Postgres"]
async fn same_adv_id_in_two_namespaces_survives_independently() {
    let db = db_url();
    let store = Store::connect(&db, 5).await.expect("connect + migrate");
    let pool = sqlx::PgPool::connect(&db).await.expect("test pool");

    let ns_a = format!("it_collision_a_{}", uuid::Uuid::new_v4().simple());
    let ns_b = format!("it_collision_b_{}", uuid::Uuid::new_v4().simple());
    let tmp_a = TempRoot::new("collision-a");
    let tmp_b = TempRoot::new("collision-b");

    tmp_a.write_wisdom(&[
        r#"{"id":"pw-1","type":"gotcha","content":"project A lesson","source_change":"ca"}"#,
    ]);
    tmp_b.write_wisdom(&[
        r#"{"id":"pw-1","type":"gotcha","content":"project B lesson","source_change":"cb"}"#,
    ]);

    reconcile_once(&store, &tmp_a.root(&ns_a)).await;
    reconcile_once(&store, &tmp_b.root(&ns_b)).await;

    // Both rows survive under the same id, each in its own namespace with its
    // own content.
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT namespace, content FROM memories WHERE id = 'pw-1' ORDER BY namespace",
    )
    .fetch_all(&pool)
    .await
    .expect("read rows");
    assert_eq!(
        rows,
        vec![
            (ns_a.clone(), "project A lesson".to_string()),
            (ns_b.clone(), "project B lesson".to_string()),
        ],
        "the same raw id must survive independently in both namespaces"
    );

    // Recall stays scoped: each namespace returns exactly its own memory.
    for (ns, expected) in [(&ns_a, "project A lesson"), (&ns_b, "project B lesson")] {
        let hits = store
            .recall(
                &vec![0.0f32; EMBEDDING_DIM],
                std::slice::from_ref(ns),
                10,
                Some(&RecallFilters::default()),
            )
            .await
            .expect("recall");
        assert_eq!(hits.len(), 1, "recall in {ns} returns exactly one row");
        assert_eq!(hits[0].id, "pw-1");
        assert_eq!(hits[0].namespace, *ns);
        assert_eq!(hits[0].content, expected);
    }

    // A promotion in B addresses only B's row. A's same-id row is untouched.
    let target = "docs/specs/0012-namespace-id-pk.md#sha256:collision";
    let updated = store
        .promote(
            "pw-1",
            &ns_b,
            &PromotionState::Promoted {
                target: target.to_string(),
            },
            PromotionStateKind::Episodic,
        )
        .await
        .expect("promote B's row");
    assert_eq!(updated, 1);
    assert_eq!(
        stored_state(&pool, &ns_b, "pw-1").await,
        Some(PromotionState::Promoted {
            target: target.to_string()
        })
    );
    assert_eq!(
        stored_state(&pool, &ns_a, "pw-1").await,
        None,
        "promoting B's row must not touch A's same-id row"
    );

    for namespace in [&ns_a, &ns_b] {
        sqlx::query("DELETE FROM memories WHERE namespace = $1")
            .bind(namespace)
            .execute(&pool)
            .await
            .expect("cleanup");
    }
}
