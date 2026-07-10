//! episode — persistent decision-memory MCP server for AI coding agents.
//!
//! Scaffold entrypoint. Subsequent phases add:
//!   - server:  rmcp stdio server + recall/remember/forget/stats tools
//!   - store:   sqlx PgPool + pgvector (HNSW cosine)
//!   - embed:   fastembed (local) / voyage (optional)
//!   - ingest:  .adv/wisdom.jsonl + reflections.jsonl watcher
//!
//! See README.md for the full architecture.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "episode starting (scaffold — MCP server lands in the next phase)"
    );

    Ok(())
}
