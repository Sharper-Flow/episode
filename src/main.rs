//! episode — binary entrypoint. All logic lives in the library crate
//! (`episode::run`) so it can be integration-tested.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // MCP speaks JSON-RPC over stdout — logs MUST go to stderr or they corrupt
    // the protocol stream.
    tracing_subscriber::fmt()
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    episode::run().await
}
