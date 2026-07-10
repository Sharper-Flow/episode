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
pub async fn run() -> Result<()> {
    let cfg = Config::from_env()?;
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

/// Periodically reconcile each project's `.adv` wisdom/reflections into the
/// memory store. Integration glue: parses (ingest), dedups per-item, embeds in
/// bounded batches through the scheduler, and upserts per-item.
///
/// Embedding is routed through the [`SchedulerHandle`] in chunks of at most
/// [`scheduler::INGEST_BATCH_MAX`] so 65+ eligible memories become multiple
/// bounded batches (AC4) and a queued `recall` is served between batches (AC5).
/// Per-item `exists_source` dedup and per-item `upsert` are intentionally
/// retained here; bulk dedup/upsert is owned by a dependent task.
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
            let adv_dir = root.path.join(".adv");
            let mut items = Vec::new();
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

            // Per-item dedup fast-path for already-ingested rows (retained; bulk
            // dedup is a dependent task).
            let mut eligible: Vec<MemoryInput> = Vec::new();
            for item in items {
                if let Some(sid) = &item.source_id {
                    match store.exists_source(&item.namespace, sid).await {
                        Ok(true) => continue,
                        Ok(false) => {}
                        Err(e) => {
                            tracing::warn!(error = %e, "exists_source failed");
                            continue;
                        }
                    }
                }
                eligible.push(item);
            }

            // Embed eligible items in bounded batches via the scheduler, then
            // upsert per-item (bulk upsert is a dependent task).
            let mut ingested = 0usize;
            let mut stopped = false;
            for (batch_idx, chunk) in eligible.chunks(scheduler::INGEST_BATCH_MAX).enumerate() {
                if *shutdown.borrow() {
                    stopped = true;
                    break;
                }
                let texts: Vec<String> = chunk.iter().map(|i| i.content.clone()).collect();
                let embeddings = match handle.embed_batch(texts).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(
                            namespace = %root.namespace,
                            batch_idx,
                            error = %e,
                            "batch embed failed"
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
                        "batch output validation failed; skipping batch"
                    );
                    continue;
                }
                for (item, emb) in chunk.iter().zip(embeddings.iter()) {
                    match store.upsert(item, emb.as_slice()).await {
                        Ok(true) => ingested += 1,
                        Ok(false) => {}
                        Err(e) => tracing::warn!(error = %e, "upsert failed"),
                    }
                }
            }
            if stopped {
                break;
            }
            if ingested > 0 {
                tracing::info!(namespace = %root.namespace, ingested, "ingested memories");
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
