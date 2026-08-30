//! Shared domain types for episode.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(crate = "rmcp::schemars")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
pub enum ActionState {
    /// Agent resolved the issue in the current session.
    AdHocResolved { summary: String },
    /// Agent linked the issue to `MemoryContext::work_id`.
    LinkedWork {},
    /// Issue remains unresolved and should surface to an operator.
    OpenFollowup {},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryContextValidationError {
    EmptyAdHocSummary,
    LinkedWorkMissingWorkId,
}

impl fmt::Display for MemoryContextValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAdHocSummary => {
                write!(f, "ad_hoc_resolved action requires a non-empty summary")
            }
            Self::LinkedWorkMissingWorkId => {
                write!(f, "linked_work action requires a non-empty context work_id")
            }
        }
    }
}

impl std::error::Error for MemoryContextValidationError {}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionState>,
}

impl MemoryContext {
    pub fn validate(&self) -> Result<(), MemoryContextValidationError> {
        match self.action.as_ref() {
            Some(ActionState::AdHocResolved { summary }) if summary.trim().is_empty() => {
                Err(MemoryContextValidationError::EmptyAdHocSummary)
            }
            Some(ActionState::LinkedWork {})
                if self
                    .work_id
                    .as_deref()
                    .is_none_or(|work_id| work_id.trim().is_empty()) =>
            {
                Err(MemoryContextValidationError::LinkedWorkMissingWorkId)
            }
            _ => Ok(()),
        }
    }
}

/// Reserved `metadata` key holding a memory's [`PromotionState`].
///
/// Sits alongside the `action` key rather than in a typed column: promotion is
/// a filterable attribute of an existing row, not a new dimension of the schema.
pub const PROMOTION_STATE_KEY: &str = "promotion_state";

/// Where a memory sits on the path from episodic recall to durable Product
/// knowledge.
///
/// The episodic default is the *absence* of [`PROMOTION_STATE_KEY`], not a
/// stored variant. Recall excludes one tagged shape and passes everything else,
/// so an explicit marker would need a backfill across every row and buy nothing.
///
/// `Superseded { by }` was considered and dropped: retracted rows are deleted
/// rather than marked, and `forget_manual` covers manual deletion, so no caller
/// could produce it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
pub enum PromotionState {
    /// Graduated in ADV, or flagged as recurring. Episode still holds the
    /// authoritative copy, so this stays visible in recall.
    PromotionCandidate {},
    /// Graduated, naming the Concord record that now owns the content.
    Promoted {
        /// Opaque to episode. Concord resolves a target by manifest path plus
        /// sha256 and publishes no serialized format, so episode stores the
        /// string verbatim and never parses it.
        target: String,
    },
}

/// The tag half of a [`PromotionState`], used as a compare-and-set precondition.
///
/// Separate from [`PromotionState`] because a transition asserts *which state a
/// row is in*, not which payload it carries: demoting a promoted row must not
/// require the caller to already know its target. [`Self::Episodic`] has no tag
/// because the episodic default is the absent key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(crate = "rmcp::schemars")]
pub enum PromotionStateKind {
    /// No stored promotion state.
    Episodic,
    PromotionCandidate,
    Promoted,
}

impl PromotionStateKind {
    /// The stored `kind` tag, or `None` when the key is absent.
    ///
    /// `None` maps to SQL `NULL`, which selects the absent-key branch in the
    /// compare-and-set statement.
    pub fn as_tag(&self) -> Option<&'static str> {
        match self {
            Self::Episodic => None,
            Self::PromotionCandidate => Some("promotion_candidate"),
            Self::Promoted => Some("promoted"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionStateValidationError {
    EmptyPromotionTarget,
}

impl fmt::Display for PromotionStateValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPromotionTarget => {
                write!(f, "promoted state requires a non-blank target")
            }
        }
    }
}

impl std::error::Error for PromotionStateValidationError {}

impl PromotionState {
    /// Non-blank is the only check episode can honestly make. Concord exposes no
    /// runtime resolution surface, so a deeper local check could not tell a real
    /// target from a plausible one.
    ///
    /// The accepted failure mode: a malformed target is excluded from episode
    /// recall *and* rejected by Concord as `knowledge_missing`. The two-step
    /// flow bounds it — nothing is hidden until a human supplies a target — and
    /// demotion recovers from it.
    pub fn validate(&self) -> Result<(), PromotionStateValidationError> {
        match self {
            Self::Promoted { target } if target.trim().is_empty() => {
                Err(PromotionStateValidationError::EmptyPromotionTarget)
            }
            _ => Ok(()),
        }
    }

    pub fn kind(&self) -> PromotionStateKind {
        match self {
            Self::PromotionCandidate {} => PromotionStateKind::PromotionCandidate,
            Self::Promoted { .. } => PromotionStateKind::Promoted,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
#[schemars(crate = "rmcp::schemars")]
pub struct RecallFilters {
    /// Scope recall to one Product: rows tagged with this Product plus every
    /// row carrying no product claim (the shared pool). Rows claiming a
    /// different Product — including an explicit `product: null` — stay
    /// excluded.
    pub product: Option<String>,
    pub work_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub kinds: Option<Vec<String>>,
    /// Narrow recall to rows from these provenances. Closed set: `manual`,
    /// `adv_wisdom`, `adv_reflection`. Unknown values reject at
    /// deserialization; an empty list rejects validation.
    pub sources: Option<Vec<MemorySource>>,
    /// Exclude rows first captured longer ago than this many days. The basis
    /// is `created_at` — first-capture time, which write-once ingest and
    /// promotion transitions never move. Pure filter: ranking and scores are
    /// untouched. Must be positive.
    pub max_age_days: Option<u32>,
    pub include_open_followups: bool,
    /// Return rows already graduated to a durable Concord record.
    ///
    /// Defaults to `false` so recall stops serving knowledge a spec now owns.
    /// `PromotionCandidate` rows are unaffected in either mode: a candidate has
    /// not graduated, so episode still holds the authoritative copy.
    pub include_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallFilterValidationError(&'static str);

impl fmt::Display for RecallFilterValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RecallFilterValidationError {}

impl RecallFilters {
    pub fn validate(&self) -> Result<(), RecallFilterValidationError> {
        if self.product.as_deref().is_some_and(|v| v.trim().is_empty()) {
            return Err(RecallFilterValidationError(
                "product filter must not be blank",
            ));
        }
        if self.work_id.as_deref().is_some_and(|v| v.trim().is_empty()) {
            return Err(RecallFilterValidationError(
                "work_id filter must not be blank",
            ));
        }
        if self
            .tags
            .as_ref()
            .is_some_and(|values| values.is_empty() || values.iter().any(|v| v.trim().is_empty()))
        {
            return Err(RecallFilterValidationError(
                "tags filter must contain non-blank values",
            ));
        }
        if self
            .kinds
            .as_ref()
            .is_some_and(|values| values.is_empty() || values.iter().any(|v| v.trim().is_empty()))
        {
            return Err(RecallFilterValidationError(
                "kinds filter must contain non-blank values",
            ));
        }
        if self
            .sources
            .as_ref()
            .is_some_and(|values| values.is_empty())
        {
            return Err(RecallFilterValidationError(
                "sources filter must not be empty",
            ));
        }
        if self.max_age_days.is_some_and(|days| days == 0) {
            return Err(RecallFilterValidationError(
                "max_age_days filter must be positive",
            ));
        }
        Ok(())
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
mod promotion_state_tests {
    use super::{PromotionState, PromotionStateValidationError};
    use serde_json::json;
    use std::collections::BTreeSet;

    /// AC1: both stored variants round-trip under the `kind` tag in snake_case.
    #[test]
    fn variant_tags_serialize_snake_case() {
        assert_eq!(
            serde_json::to_value(PromotionState::PromotionCandidate {})
                .expect("candidate must serialize"),
            json!({ "kind": "promotion_candidate" })
        );
        assert_eq!(
            serde_json::to_value(PromotionState::Promoted {
                target: "spec/0011#sha256:abc".to_string(),
            })
            .expect("promoted must serialize"),
            json!({ "kind": "promoted", "target": "spec/0011#sha256:abc" })
        );
    }

    #[test]
    fn variant_tags_deserialize_round_trip() {
        let candidate: PromotionState =
            serde_json::from_value(json!({ "kind": "promotion_candidate" }))
                .expect("candidate must deserialize");
        assert_eq!(candidate, PromotionState::PromotionCandidate {});

        let promoted: PromotionState =
            serde_json::from_value(json!({ "kind": "promoted", "target": "t" }))
                .expect("promoted must deserialize");
        assert_eq!(
            promoted,
            PromotionState::Promoted {
                target: "t".to_string()
            }
        );
    }

    /// AC1: `Episodic` is the absent-key default and must never be a variant.
    /// A stored literal would force a backfill over every existing row.
    #[test]
    fn episodic_is_not_a_variant() {
        let parsed = serde_json::from_value::<PromotionState>(json!({ "kind": "episodic" }));
        assert!(
            parsed.is_err(),
            "episodic must not be a stored variant; absence of the key is the default"
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let parsed = serde_json::from_value::<PromotionState>(
            json!({ "kind": "promotion_candidate", "extra": 1 }),
        );
        assert!(parsed.is_err(), "deny_unknown_fields must reject extras");
    }

    /// AC8: `target` is opaque. Validation is non-blank and nothing more —
    /// Concord publishes no serialized format and exposes no resolution surface,
    /// so any deeper local check would be theater.
    #[test]
    fn validate_rejects_blank_target() {
        for blank in ["", "   ", "\t\n"] {
            let state = PromotionState::Promoted {
                target: blank.to_string(),
            };
            assert_eq!(
                state.validate(),
                Err(PromotionStateValidationError::EmptyPromotionTarget),
                "blank target {blank:?} must be rejected"
            );
        }
    }

    #[test]
    fn validate_accepts_opaque_nonblank_target() {
        // Deliberately unparseable shapes: episode must not infer structure.
        for target in [
            "x",
            "docs/decisions/CD-0002.md#sha256:deadbeef",
            "{\"a\":1}",
        ] {
            let state = PromotionState::Promoted {
                target: target.to_string(),
            };
            assert_eq!(
                state.validate(),
                Ok(()),
                "opaque target {target:?} must pass"
            );
        }
        assert_eq!(PromotionState::PromotionCandidate {}.validate(), Ok(()));
    }

    /// AC1: the empty-payload variant must stay closed under schemars, matching
    /// the `LinkedWork {}` / `OpenFollowup {}` precedent verified in server.rs.
    #[test]
    fn schema_variants_are_closed_and_exact() {
        let schema = serde_json::to_value(rmcp::schemars::schema_for!(PromotionState))
            .expect("promotion state schema must serialize");
        let variants = schema
            .get("oneOf")
            .or_else(|| schema.get("anyOf"))
            .and_then(serde_json::Value::as_array)
            .expect("schema must expose variant branches");

        let actual = variants
            .iter()
            .filter_map(|v| v["properties"]["kind"]["const"].as_str())
            .collect::<BTreeSet<_>>();
        let expected = ["promotion_candidate", "promoted"]
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);

        for variant in variants {
            assert_eq!(
                variant["additionalProperties"],
                json!(false),
                "each promotion state variant must be closed"
            );
        }
    }
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
