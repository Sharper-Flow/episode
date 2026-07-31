//! episode — binary entrypoint. All logic lives in the library crate
//! (`episode::run`) so it can be integration-tested.

use anyhow::{Context, Result};
use episode::config::Config;
use tracing_subscriber::filter::LevelFilter;

/// Initialize the stderr-only subscriber used by the MCP server.
///
/// stdout is reserved for JSON-RPC, so logs MUST go to stderr. The level is
/// injected from the validated config seam (default INFO) so it is testable in
/// isolation via `Config::from_vars`.
fn init_logging(level: LevelFilter) -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|e| anyhow::anyhow!("failed to initialize tracing subscriber: {e}"))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // Parse config before initializing logging so the configured level is honored.
    // If config parsing fails, the error is returned to the caller via stderr by
    // the default `anyhow` main error path; no logs are emitted before init.
    let cfg = Config::from_env().context("configuration failed")?;
    init_logging(cfg.log_level)?;

    episode::run(cfg).await
}
