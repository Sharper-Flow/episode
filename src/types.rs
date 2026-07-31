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

#[cfg(test)]
mod tests {
    use super::EMBEDDING_DIM;

    const INIT_SQL: &str = include_str!("../migrations/0001_init.sql");

    fn extract_embedding_dim(sql: &str) -> Option<usize> {
        for line in sql.lines() {
            // Strip inline comments before matching.
            let line = line.split("--").next()?;
            let lower = line.to_ascii_lowercase();
            if lower.contains("embedding") && lower.contains("vector(") {
                let open = lower.find("vector(")? + 7;
                let close = lower[open..].find(")")? + open;
                return line[open..close].trim().parse().ok();
            }
        }
        None
    }

    fn extract_hnsw_index(sql: &str) -> Option<&str> {
        let start = sql.find("CREATE INDEX IF NOT EXISTS memories_embedding_hnsw")?;
        let end = sql[start..].find(';')? + start;
        Some(&sql[start..=end])
    }

    /// AC1 / SC1: the Rust embedding-dimension constant must agree with the
    /// persisted schema. This test is database-free — it parses the embedded
    /// migration SQL and compares it to `EMBEDDING_DIM`. A mismatch means the
    /// backend could produce vectors that the Postgres column would reject.
    #[test]
    fn embedding_dimension_matches_schema() {
        let dim = extract_embedding_dim(INIT_SQL)
            .expect("init schema must declare an embedding vector(N) dimension");
        assert_eq!(
            dim, EMBEDDING_DIM,
            "EMBEDDING_DIM constant must equal the schema's vector(N) dimension"
        );
    }

    /// AC6: the HNSW index must use cosine distance and state the pgvector
    /// default parameters explicitly. Fresh databases get the documented shape
    /// from the initial migration; existing databases keep their creation-time
    /// parameters, because changing HNSW build parameters requires recreating
    /// the index (a data migration we deliberately avoid).
    #[test]
    fn hnsw_index_uses_cosine_with_explicit_defaults() {
        let idx = extract_hnsw_index(INIT_SQL)
            .expect("init schema must declare a memories_embedding_hnsw index");
        let normalized = idx.to_ascii_lowercase();
        assert!(
            normalized.contains("vector_cosine_ops"),
            "HNSW index must use cosine distance (vector_cosine_ops), got:\n{idx}"
        );
        assert!(
            normalized.contains("m = 16"),
            "HNSW index must explicitly document m = 16 (pgvector default), got:\n{idx}"
        );
        assert!(
            normalized.contains("ef_construction = 64"),
            "HNSW index must explicitly document ef_construction = 64 (pgvector default), got:\n{idx}"
        );
    }
}
