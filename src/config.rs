//! Environment-driven configuration.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// A project root to ingest ADV wisdom/reflections from, with its namespace.
#[derive(Debug, Clone)]
pub struct ProjectRoot {
    pub namespace: String,
    pub path: PathBuf,
}

/// Embedding backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedBackend {
    /// Local fastembed (default).
    Local,
    /// Voyage API (`voyage-4-lite`).
    Voyage,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub pool_size: u32,
    pub embed_backend: EmbedBackend,
    pub voyage_api_key: Option<String>,
    pub project_roots: Vec<ProjectRoot>,
    /// Seconds between ingestion reconcile passes.
    pub ingest_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let database_url =
            std::env::var("EPISODE_DATABASE_URL").context("EPISODE_DATABASE_URL must be set")?;

        let pool_size = std::env::var("EPISODE_DB_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);

        let embed_backend = match std::env::var("EPISODE_EMBED_BACKEND")
            .unwrap_or_else(|_| "local".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "voyage" => EmbedBackend::Voyage,
            _ => EmbedBackend::Local,
        };

        let voyage_api_key = std::env::var("VOYAGE_API_KEY").ok();

        let project_roots = std::env::var("EPISODE_PROJECT_ROOTS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|pair| {
                let pair = pair.trim();
                if pair.is_empty() {
                    return None;
                }
                let (ns, path) = pair.split_once('=')?;
                Some(ProjectRoot {
                    namespace: ns.trim().to_string(),
                    path: PathBuf::from(path.trim()),
                })
            })
            .collect();

        let ingest_interval_secs = std::env::var("EPISODE_INGEST_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);

        Ok(Self {
            database_url,
            pool_size,
            embed_backend,
            voyage_api_key,
            project_roots,
            ingest_interval_secs,
        })
    }
}
