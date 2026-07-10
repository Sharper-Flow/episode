//! Shared domain types for episode.

use serde::{Deserialize, Serialize};

/// Embedding dimensionality. Fits both fastembed (BGE-large / BGE-M3) and
/// voyage-4-lite, so the backend can switch without a schema migration.
pub const EMBEDDING_DIM: usize = 1024;

/// Provenance of a memory row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    /// Ingested from `{project}/.adv/wisdom.jsonl`.
    AdvWisdom,
    /// Ingested from `{project}/.adv/reflections.jsonl`.
    AdvReflection,
    /// Written directly via the `remember` MCP tool.
    Manual,
}

impl MemorySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemorySource::AdvWisdom => "adv_wisdom",
            MemorySource::AdvReflection => "adv_reflection",
            MemorySource::Manual => "manual",
        }
    }
}

/// An item ready to be embedded and upserted. Produced by ingestion parsing
/// or the `remember` tool; consumed by the store.
#[derive(Debug, Clone)]
pub struct MemoryInput {
    /// Stable id — mirrors the ADV source id (`pw-*` / `rf-*`) for ingested
    /// rows, or `mem-*` for manual writes.
    pub id: String,
    pub namespace: String,
    pub source: MemorySource,
    /// Original ADV id for dedup; `None` for manual writes.
    pub source_id: Option<String>,
    /// Wisdom type (pattern|gotcha|convention|success|failure) or reflection
    /// facet (friction|highlight|suggestion). `None` allowed.
    pub kind: Option<String>,
    pub content: String,
    pub metadata: serde_json::Value,
}

/// A recall result row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallHit {
    pub id: String,
    pub namespace: String,
    pub source: String,
    pub kind: Option<String>,
    pub content: String,
    /// Cosine similarity in [0, 1] (1 = identical). Derived from `1 - distance`.
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// A namespace + source count pair for `stats`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceStat {
    pub namespace: String,
    pub source: String,
    pub count: i64,
}
