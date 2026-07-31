//! episode — persistent decision-memory MCP server for AI coding agents.
//!
//! Library crate: modules are public so integration tests and the thin binary
//! (`src/main.rs`) can drive them.

pub mod config;
pub mod embed;
pub mod ingest;
pub mod scheduler;
pub mod server;
pub mod store;
pub mod types;

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use tokio::sync::watch;

use crate::config::{Config, EmbedBackend, ProjectRoot};
use crate::embed::{Embedder, LocalEmbedder};
use crate::scheduler::{NoopRecorder, SchedulerConfig, SchedulerHandle};
use crate::server::EpisodeServer;
use crate::store::Store;
use crate::types::MemoryInput;

/// Boot episode: load config, connect the store, build the embedder behind the
/// bounded priority scheduler, spawn the background ingestion loop, and serve
/// the MCP protocol over stdio. Owns the ingestion and scheduler task handles
/// and drives cooperative shutdown when stdio completes (AC6 / C5).
pub async fn run(cfg: Config) -> Result<()> {
    tracing::info!(
        roots = cfg.project_roots.len(),
        pool = cfg.pool_size,
        "episode starting"
    );

    let store = Store::connect(&cfg.database_url, cfg.pool_size).await?;

    // Exactly one embedder/model is constructed and moved into the scheduler;
    // the server and ingestion loop only see a cloneable handle (C4).
    let embedder: Arc<dyn Embedder> = match cfg.embed_backend {
        EmbedBackend::Local => Arc::new(LocalEmbedder::new()?),
        EmbedBackend::Voyage => {
            anyhow::bail!("voyage embedding backend is not implemented in v0")
        }
    };

    // In-tree cooperative shutdown signal shared with the scheduler worker and
    // the ingestion loop. `run()` keeps the sender and flips it after stdio
    // completes; both tasks exit after any active work returns.
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (handle, worker_handle) = scheduler::start(
        embedder,
        SchedulerConfig::default(),
        Arc::new(NoopRecorder),
        shutdown_rx.clone(),
    );

    // Background ingestion of ADV wisdom/reflections. The JoinHandle is
    // retained so `run()` can await clean termination (AC6).
    let ingest_handle = {
        let store = store.clone();
        let handle = handle.clone();
        let roots = cfg.project_roots.clone();
        let interval = cfg.ingest_interval_secs;
        let rx = shutdown_rx.clone();
        tokio::spawn(async move {
            run_ingestion_loop(store, handle, roots, interval, rx).await;
        })
    };

    // Serve stdio, then always drive cooperative shutdown on every path.
    let outcome = async {
        let service = EpisodeServer::new(store, handle).serve(stdio()).await?;
        service.waiting().await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    let _ = shutdown_tx.send(true);
    if let Err(e) = ingest_handle.await {
        tracing::warn!(error = %e, "ingestion task join failed");
    }
    if let Err(e) = worker_handle.await {
        tracing::warn!(error = %e, "embedding scheduler join failed");
    }
    outcome
}

/// Collect the `source_id`s of ingested items for one bulk dedup lookup. Items
/// without a `source_id` (manual writes) never appear here: they are always
/// eligible and are never deduped by `source_id`.
fn ingest_source_ids(items: &[MemoryInput]) -> Vec<String> {
    items.iter().filter_map(|i| i.source_id.clone()).collect()
}

/// Drop items already present in `existing` (by `source_id`); preserve stable
/// ids — including exploded reflection children like `rf-1:friction:0` — and keep
/// items with no `source_id`. Input order is preserved.
fn filter_eligible(items: Vec<MemoryInput>, existing: &HashSet<String>) -> Vec<MemoryInput> {
    items
        .into_iter()
        .filter(|i| match &i.source_id {
            Some(sid) => !existing.contains(sid),
            None => true,
        })
        .collect()
}

/// Half-open index ranges partitioning `total` items into chunks of at most
/// `max`. 65 items with `max = 64` yields `[0..64, 64..65]` (AC4: multiple
/// bounded batches, never one unbounded call). An empty input — or a degenerate
/// `max == 0` — yields no ranges.
fn partition_ranges(total: usize, max: usize) -> Vec<Range<usize>> {
    if max == 0 || total == 0 {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < total {
        let end = (start + max).min(total);
        ranges.push(start..end);
        start = end;
    }
    ranges
}

/// Outcome of reconciling one project root. `stopped` is set when shutdown was
/// observed before a batch began, so the caller can break the root loop.
struct ReconcileOutcome {
    ingested: usize,
    stopped: bool,
}

/// Reconcile one project root: parse both ADV sources, bulk-dedup against the
/// store, then embed and persist eligible items in bounded, transactional
/// partitions.
///
/// - **Bulk dedup (design §4):** one `existing_source_ids` lookup per root
///   replaces the previous per-item `exists_source` calls.
/// - **Bounded batches (AC4):** eligible items are partitioned into chunks of at
///   most [`scheduler::INGEST_BATCH_MAX`], so 65+ items become multiple batches.
/// - **Transactional persistence (DONT3):** each partition is persisted through
///   [`Store::upsert_batch`] in a single transaction using `QueryBuilder` binds.
/// - **Failed-partition isolation:** an embed, validation, or upsert failure logs
///   the namespace and `continue`s to the next partition; the failed partition's
///   items remain unpersisted and therefore reappear on the next reconcile. A
///   bulk-lookup failure skips the whole root for this pass (nothing is
///   persisted, so everything reconciles next time).
async fn reconcile_root(
    store: &Store,
    handle: &SchedulerHandle,
    root: &ProjectRoot,
    shutdown: &watch::Receiver<bool>,
) -> ReconcileOutcome {
    let adv_dir = root.path.join(".adv");
    let mut items: Vec<MemoryInput> = Vec::new();
    match ingest::parse_wisdom(&root.namespace, &adv_dir) {
        Ok(mut w) => items.append(&mut w),
        Err(e) => {
            tracing::warn!(namespace = %root.namespace, error = %e, "wisdom parse failed")
        }
    }
    match ingest::parse_reflections(&root.namespace, &adv_dir) {
        Ok(mut r) => items.append(&mut r),
        Err(e) => {
            tracing::warn!(namespace = %root.namespace, error = %e, "reflection parse failed")
        }
    }

    // One static-SQL bulk lookup per root replaces per-item EXISTS dedup.
    let source_ids = ingest_source_ids(&items);
    let existing = match store
        .existing_source_ids(&root.namespace, &source_ids)
        .await
    {
        Ok(set) => set,
        Err(e) => {
            tracing::warn!(
                namespace = %root.namespace,
                error = %e,
                "bulk source-id lookup failed; skipping root this pass"
            );
            return ReconcileOutcome {
                ingested: 0,
                stopped: false,
            };
        }
    };
    let eligible = filter_eligible(items, &existing);

    let mut ingested = 0usize;
    let ranges = partition_ranges(eligible.len(), scheduler::INGEST_BATCH_MAX);
    for (batch_idx, range) in ranges.iter().enumerate() {
        // Cooperative stop (AC6 / C5): never start a new batch after shutdown.
        if *shutdown.borrow() {
            return ReconcileOutcome {
                ingested,
                stopped: true,
            };
        }
        let chunk = &eligible[range.clone()];
        let texts: Vec<String> = chunk.iter().map(|i| i.content.clone()).collect();
        let embeddings = match handle.embed_batch(texts).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    namespace = %root.namespace,
                    batch_idx,
                    error = %e,
                    "batch embed failed; partition left for next reconcile"
                );
                continue;
            }
        };
        if let Err(e) = scheduler::validate_batch_output(chunk.len(), &embeddings) {
            tracing::error!(
                namespace = %root.namespace,
                batch_idx,
                expected = chunk.len(),
                got = embeddings.len(),
                error = %e,
                "batch output validation failed; skipping partition"
            );
            continue;
        }
        // Transactional persist of the whole partition. A failure rolls back and
        // logs the namespace without poisoning later partitions or roots.
        match store.upsert_batch(chunk, &embeddings).await {
            Ok(n) => ingested += n,
            Err(e) => {
                tracing::warn!(
                    namespace = %root.namespace,
                    batch_idx,
                    error = %e,
                    "batch upsert failed; partition rolled back and left for next reconcile"
                );
                continue;
            }
        }
    }
    ReconcileOutcome {
        ingested,
        stopped: false,
    }
}

/// Periodically reconcile each project's `.adv` wisdom/reflections into the
/// memory store. Integration glue: parses (ingest), bulk-dedups per root, embeds
/// in bounded batches through the scheduler, and persists each partition
/// transactionally.
///
/// Embedding is routed through the [`SchedulerHandle`] in chunks of at most
/// [`scheduler::INGEST_BATCH_MAX`] so 65+ eligible memories become multiple
/// bounded batches (AC4) and a queued `recall` is served between batches (AC5).
/// Dedup uses one bulk `existing_source_ids` lookup per root and persistence
/// uses one transactional `upsert_batch` per partition (design §4).
///
/// Cancellation is cooperative (AC6 / C5): the `shutdown` watch is checked at
/// the top of each pass, between roots, and before each batch begins, and the
/// interval sleep is cancellation-aware. Active embedding may finish; no new
/// batch starts after shutdown. `JoinHandle::abort` is never used.
pub async fn run_ingestion_loop(
    store: Store,
    handle: SchedulerHandle,
    roots: Vec<ProjectRoot>,
    interval_secs: u64,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        if *shutdown.borrow() {
            break;
        }
        for root in &roots {
            if *shutdown.borrow() {
                break;
            }
            let outcome = reconcile_root(&store, &handle, root, &shutdown).await;
            if outcome.stopped {
                break;
            }
            if outcome.ingested > 0 {
                tracing::info!(namespace = %root.namespace, ingested = outcome.ingested, "ingested memories");
            }
        }

        // Cancellation-aware interval: return immediately on shutdown instead
        // of sleeping through it.
        tokio::select! {
            _ = shutdown.changed() => break,
            _ = tokio::time::sleep(Duration::from_secs(interval_secs)) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemorySource;

    /// An ingested ADV item whose `source_id` equals its stable `id` (the common
    /// case for wisdom entries and exploded reflection children).
    fn adv(id: &str, ns: &str) -> MemoryInput {
        MemoryInput {
            id: id.to_string(),
            namespace: ns.to_string(),
            source: MemorySource::AdvWisdom,
            source_id: Some(id.to_string()),
            kind: Some("gotcha".into()),
            content: format!("content-{id}"),
            metadata: serde_json::json!({}),
        }
    }

    /// A manual `remember` item: no `source_id`, so it is always eligible and is
    /// never deduped by `source_id`.
    fn manual(id: &str, ns: &str) -> MemoryInput {
        MemoryInput {
            id: id.to_string(),
            namespace: ns.to_string(),
            source: MemorySource::Manual,
            source_id: None,
            kind: None,
            content: format!("content-{id}"),
            metadata: serde_json::json!({}),
        }
    }

    // ---- AC4 / DONT3: bounded partitioning -------------------------------

    #[test]
    fn partition_65_yields_two_bounded_batches() {
        let ranges = partition_ranges(65, scheduler::INGEST_BATCH_MAX);
        assert_eq!(
            ranges,
            vec![0..64, 64..65],
            "65 items must split into 64 + 1"
        );
        assert_eq!(
            ranges.len(),
            2,
            "AC4: 65 eligible items make multiple batches, not one unbounded call"
        );
        assert!(
            ranges
                .iter()
                .all(|r| r.len() <= scheduler::INGEST_BATCH_MAX),
            "no partition may exceed INGEST_BATCH_MAX"
        );
    }

    #[test]
    fn partition_boundaries() {
        let empty: Vec<Range<usize>> = Vec::new();
        assert_eq!(partition_ranges(0, 64), empty);
        assert_eq!(partition_ranges(1, 64), vec![0..1]);
        assert_eq!(partition_ranges(64, 64), vec![0..64]);
        assert_eq!(partition_ranges(128, 64), vec![0..64, 64..128]);
        assert_eq!(
            partition_ranges(200, 64),
            vec![0..64, 64..128, 128..192, 192..200]
        );
        assert_eq!(
            partition_ranges(65, 0),
            empty,
            "max == 0 is degenerate and yields no ranges"
        );
    }

    // ---- design §4: bulk dedup inputs + stable-id preservation -----------

    #[test]
    fn ingest_source_ids_skips_manual_and_preserves_order() {
        let items = vec![adv("pw-1", "n"), manual("mem-1", "n"), adv("pw-2", "n")];
        assert_eq!(
            ingest_source_ids(&items),
            vec!["pw-1".to_string(), "pw-2".to_string()],
            "manual items (no source_id) are excluded; order is preserved"
        );
        assert!(ingest_source_ids(&[manual("mem-2", "n")]).is_empty());
    }

    #[test]
    fn filter_eligible_dedups_and_preserves_stable_reflection_ids() {
        let existing: HashSet<String> = ["pw-1".to_string()].into_iter().collect();
        let items = vec![
            adv("pw-1", "n"),               // already ingested -> dropped
            adv("rf-xyz:friction:0", "n"),  // exploded reflection child -> kept
            adv("rf-xyz:highlight:0", "n"), // reflection sibling -> kept
            manual("mem-1", "n"),           // no source_id -> kept
        ];
        let got = filter_eligible(items, &existing);
        let ids: Vec<&str> = got.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["rf-xyz:friction:0", "rf-xyz:highlight:0", "mem-1"],
            "already-ingested items are dropped; the rest keep input order"
        );
        // Stable ids — including exploded reflection children — are preserved
        // verbatim through the dedup filter.
        assert_eq!(got[0].id, "rf-xyz:friction:0");
        assert_eq!(got[0].source_id.as_deref(), Some("rf-xyz:friction:0"));
        assert!(got[2].source_id.is_none(), "manual item keeps no source_id");
    }

    #[test]
    fn filter_eligible_empty_existing_keeps_all() {
        let existing = HashSet::new();
        let items = vec![adv("pw-1", "n"), adv("pw-2", "n")];
        assert_eq!(filter_eligible(items, &existing).len(), 2);
    }
}
