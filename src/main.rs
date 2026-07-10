//! episode — persistent decision-memory MCP server for AI coding agents.

mod config;
mod embed;
mod ingest;
mod server;
mod store;
mod types;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use rmcp::transport::stdio;
use rmcp::ServiceExt;

use crate::config::{Config, EmbedBackend, ProjectRoot};
use crate::embed::{Embedder, LocalEmbedder};
use crate::server::EpisodeServer;
use crate::store::Store;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // MCP speaks JSON-RPC over stdout — logs MUST go to stderr or they corrupt
    // the protocol stream.
    tracing_subscriber::fmt()
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let cfg = Config::from_env()?;
    tracing::info!(
        roots = cfg.project_roots.len(),
        pool = cfg.pool_size,
        "episode starting"
    );

    let store = Store::connect(&cfg.database_url, cfg.pool_size).await?;

    let embedder: Arc<dyn Embedder> = match cfg.embed_backend {
        EmbedBackend::Local => Arc::new(LocalEmbedder::new()?),
        EmbedBackend::Voyage => {
            anyhow::bail!("voyage embedding backend is not implemented in v0")
        }
    };

    // Background ingestion of ADV wisdom/reflections.
    {
        let store = store.clone();
        let embedder = embedder.clone();
        let roots = cfg.project_roots.clone();
        let interval = cfg.ingest_interval_secs;
        tokio::spawn(async move {
            run_ingestion_loop(store, embedder, roots, interval).await;
        });
    }

    let service = EpisodeServer::new(store, embedder)
        .serve(stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}

/// Periodically reconcile each project's `.adv` wisdom/reflections into the
/// memory store. Integration glue: parses (ingest), dedups + embeds + upserts.
async fn run_ingestion_loop(
    store: Store,
    embedder: Arc<dyn Embedder>,
    roots: Vec<ProjectRoot>,
    interval_secs: u64,
) {
    loop {
        for root in &roots {
            let adv_dir = root.path.join(".adv");
            let mut items = Vec::new();
            match ingest::parse_wisdom(&root.namespace, &adv_dir) {
                Ok(mut w) => items.append(&mut w),
                Err(e) => tracing::warn!(namespace = %root.namespace, error = %e, "wisdom parse failed"),
            }
            match ingest::parse_reflections(&root.namespace, &adv_dir) {
                Ok(mut r) => items.append(&mut r),
                Err(e) => tracing::warn!(namespace = %root.namespace, error = %e, "reflection parse failed"),
            }

            let mut ingested = 0usize;
            for item in items {
                // Dedup fast-path for ingested rows.
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

                let embedder2 = embedder.clone();
                let content = item.content.clone();
                let embedding =
                    match tokio::task::spawn_blocking(move || embedder2.embed_one(&content)).await {
                        Ok(Ok(v)) => v,
                        Ok(Err(e)) => {
                            tracing::warn!(error = %e, "embed failed");
                            continue;
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "embed task join failed");
                            continue;
                        }
                    };

                match store.upsert(&item, &embedding).await {
                    Ok(true) => ingested += 1,
                    Ok(false) => {}
                    Err(e) => tracing::warn!(error = %e, "upsert failed"),
                }
            }
            if ingested > 0 {
                tracing::info!(namespace = %root.namespace, ingested, "ingested memories");
            }
        }
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}
