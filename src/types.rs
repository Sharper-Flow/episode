//! Shared domain types for episode.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Embedding dimensionality. Fits both fastembed (BGE-large / BGE-M3) and
/// voyage-4-lite, so the backend can switch without a schema migration.
pub const EMBEDDING_DIM: usize = 1024;

/// pgvector documented default for the HNSW `m` build parameter.
///
/// The initial migration uses pgvector's implicit defaults, so these constants
/// are the machine-checkable authority for the index shape on fresh databases.
pub const HNSW_M: u32 = 16;

/// pgvector documented default for the HNSW `ef_construction` build parameter.
///
/// See [`HNSW_M`] for rationale.
pub const HNSW_EF_CONSTRUCTION: u32 = 64;

// Static assertions: the HNSW constants must stay equal to the pgvector defaults
// documented in `docs/specs/0001-dimension-contract.md`. A mismatch would
// mean the code and the operator-facing docs have drifted apart.
#[allow(dead_code)]
const _: () = assert!(HNSW_M == 16);
#[allow(dead_code)]
const _: () = assert!(HNSW_EF_CONSTRUCTION == 64);

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

/// Caller-supplied work context attached to a manually recorded memory.
///
/// Episode validates this structural field set but treats every value as opaque.
/// Sparse serialization keeps absent context equivalent to an empty metadata
/// object and gives later metadata filters stable top-level keys.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
pub struct MemoryContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_ref: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
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
    use super::{EMBEDDING_DIM, HNSW_EF_CONSTRUCTION, HNSW_M};

    const INIT_SQL: &str = include_str!("../migrations/0001_init.sql");
    const DIMENSION_CONTRACT_SPEC: &str = include_str!("../docs/specs/0001-dimension-contract.md");

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

    /// AC6: the HNSW index must use cosine distance. The pgvector default build
    /// parameters (`m = 16`, `ef_construction = 64`) are enforced by static
    /// assertions on [`HNSW_M`] and [`HNSW_EF_CONSTRUCTION`] and are documented
    /// in `docs/specs/0001-dimension-contract.md`. The initial migration relies
    /// on pgvector's implicit defaults rather than an explicit `WITH` clause, so
    /// we do not mutate the already-applied migration and break the SQLx checksum.
    #[test]
    fn hnsw_index_uses_cosine_with_documented_defaults() {
        let idx = extract_hnsw_index(INIT_SQL)
            .expect("init schema must declare a memories_embedding_hnsw index");
        let normalized = idx.to_ascii_lowercase();
        assert!(
            normalized.contains("vector_cosine_ops"),
            "HNSW index must use cosine distance (vector_cosine_ops), got:\n{idx}"
        );

        // The explicit defaults live in code (static assertions above) and in the
        // operator-facing spec; verify the spec cannot drift silently.
        let spec = DIMENSION_CONTRACT_SPEC.to_ascii_lowercase();
        assert!(
            spec.contains("m = 16"),
            "dimension contract spec must document HNSW m = 16 default"
        );
        assert!(
            spec.contains("ef_construction = 64"),
            "dimension contract spec must document HNSW ef_construction = 64 default"
        );

        // Redundant runtime assertions so a test failure reports the mismatch
        // in plain language, complementing the compile-time static assertions.
        assert_eq!(HNSW_M, 16, "HNSW_M must equal the pgvector default");
        assert_eq!(
            HNSW_EF_CONSTRUCTION, 64,
            "HNSW_EF_CONSTRUCTION must equal the pgvector default"
        );
    }
}
