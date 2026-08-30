//! rmcp server exposing recall / remember / forget / stats tools.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use serde::Deserialize;

use crate::scheduler::SchedulerHandle;
use crate::store::Store;
use crate::types::{
    MemoryContext, MemoryInput, MemorySource, PromotionState, PromotionStateKind, RecallFilters,
};

#[derive(Clone)]
pub struct EpisodeServer {
    store: Store,
    handle: SchedulerHandle,
    // Read by the `#[tool_handler]`-generated dispatch; not seen by dead-code analysis.
    #[allow(dead_code)]
    tool_router: ToolRouter<EpisodeServer>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RecallParams {
    /// Natural-language query describing the decision/context you need.
    query: String,
    /// Project namespace to search (its memories plus the shared `global`
    /// namespace). Omit to search across all namespaces.
    #[serde(default)]
    namespace: Option<String>,
    /// Maximum number of hits to return (default 8).
    #[serde(default)]
    top_k: Option<i64>,
    /// Optional structured constraints applied with AND semantics.
    #[serde(default)]
    filters: Option<RecallFilters>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RememberParams {
    /// The memory content to store (a decision, gotcha, convention, etc.).
    content: String,
    /// Namespace to store under. Defaults to `global` (cross-project).
    #[serde(default)]
    namespace: Option<String>,
    /// Optional category label (e.g. gotcha, convention, decision).
    #[serde(default)]
    kind: Option<String>,
    /// Optional caller-owned product, work, repository, tag, and severity context.
    #[serde(default)]
    context: Option<MemoryContext>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ForgetParams {
    /// The memory id to remove.
    id: String,
    /// Namespace that owns the memory. Required: deletion is a restricted hard
    /// delete that only removes a matching `manual` row in this namespace;
    /// ingested memories and other namespaces are never affected.
    namespace: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PromoteParams {
    /// The memory id to transition.
    id: String,
    /// Namespace that owns the memory.
    namespace: String,
    /// State to move the memory into.
    to: PromotionState,
    /// State kind the memory is expected to have right now. A kind mismatch
    /// changes nothing and returns `updated: 0`. Transitions from the same kind
    /// are last-writer-wins.
    from: PromotionStateKind,
}

fn internal(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}

fn context_to_metadata(context: Option<MemoryContext>) -> Result<serde_json::Value, ErrorData> {
    match context {
        Some(context) => {
            context
                .validate()
                .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
            serde_json::to_value(context).map_err(internal)
        }
        None => Ok(serde_json::json!({})),
    }
}

#[tool_router]
impl EpisodeServer {
    pub fn new(store: Store, handle: SchedulerHandle) -> Self {
        Self {
            store,
            handle,
            tool_router: Self::tool_router(),
        }
    }

    async fn embed_query(&self, text: String) -> Result<Vec<f32>, ErrorData> {
        self.handle.embed_one(text).await.map_err(internal)
    }

    #[tool(
        description = "Semantically recall durable decision memories (gotchas, conventions, failed approaches, reflections) relevant to a query. Namespace-scoped; results include a similarity score."
    )]
    async fn recall(
        &self,
        Parameters(p): Parameters<RecallParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if let Some(filters) = p.filters.as_ref() {
            filters
                .validate()
                .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
        }
        let embedding = self.embed_query(p.query).await?;
        let namespaces = match p.namespace {
            Some(ns) => vec![ns, "global".to_string()],
            None => Vec::new(),
        };
        let top_k = p.top_k.unwrap_or(8).clamp(1, 50);
        let hits = self
            .store
            .recall(&embedding, &namespaces, top_k, p.filters.as_ref())
            .await
            .map_err(internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::json(&hits)?]))
    }

    #[tool(
        description = "Store a new decision memory directly. Use for durable, cross-session learnings that should be recalled later. Defaults to the shared `global` namespace."
    )]
    async fn remember(
        &self,
        Parameters(p): Parameters<RememberParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let metadata = context_to_metadata(p.context)?;
        let embedding = self.embed_query(p.content.clone()).await?;
        let id = format!("mem-{}", uuid::Uuid::new_v4().simple());
        let input = MemoryInput {
            id: id.clone(),
            namespace: p.namespace.unwrap_or_else(|| "global".to_string()),
            source: MemorySource::Manual,
            source_id: None,
            kind: p.kind,
            content: p.content,
            metadata,
        };
        self.store
            .upsert(&input, &embedding)
            .await
            .map_err(internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::json(
            serde_json::json!({ "stored": true, "id": id }),
        )?]))
    }

    #[tool(
        description = "Restricted hard deletion of a manual memory by id within a namespace. Only the row matching id + namespace with source `manual` is removed; ingested memories and other namespaces are unaffected. Returns the `removed` count (0 or 1)."
    )]
    async fn forget(
        &self,
        Parameters(p): Parameters<ForgetParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let removed = self
            .store
            .forget_manual(&p.id, &p.namespace)
            .await
            .map_err(internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::json(
            serde_json::json!({ "removed": removed }),
        )?]))
    }

    #[tool(
        description = "Move a memory along the promotion path: flag it as a promotion candidate, record that it graduated into a durable Concord spec/decision, or demote it back. Promoted memories are excluded from `recall` by default, so the durable record and episode cannot serve conflicting copies. Works on ingested and manual memories alike. `from` is the state kind you expect the memory to have; a kind mismatch changes nothing and returns `updated: 0`. Transitions from the same kind are last-writer-wins. Returns the `updated` count (0 or 1)."
    )]
    async fn promote(
        &self,
        Parameters(p): Parameters<PromoteParams>,
    ) -> Result<CallToolResult, ErrorData> {
        p.to.validate()
            .map_err(|error| ErrorData::invalid_params(error.to_string(), None))?;
        let updated = self
            .store
            .promote(&p.id, &p.namespace, &p.to, p.from)
            .await
            .map_err(internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::json(
            serde_json::json!({ "updated": updated }),
        )?]))
    }

    #[tool(description = "Return memory counts grouped by namespace and source.")]
    async fn stats(&self) -> Result<CallToolResult, ErrorData> {
        let stats = self.store.stats().await.map_err(internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::json(&stats)?]))
    }
}

#[tool_handler]
impl ServerHandler for EpisodeServer {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo::new` fills server_info via rmcp's `from_build_env`, which
        // reports "rmcp" — override it with our own identity.
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "episode: persistent decision memory for agents. Use `recall` before starting \
                 work to surface prior gotchas/conventions/decisions; use `remember` to store \
                 durable learnings. Namespaces are per-project plus a shared `global`.",
            );
        info.server_info.name = "episode".to_string();
        info.server_info.version = env!("CARGO_PKG_VERSION").to_string();
        info
    }
}

#[cfg(test)]
mod tests {
    use super::{RecallFilters, RememberParams, context_to_metadata};
    use crate::types::MemoryContext;
    use serde_json::{Value, json};
    use std::collections::BTreeSet;

    fn resolve_object_schema<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let name = reference
                .strip_prefix("#/$defs/")
                .expect("schema reference must target $defs");
            return &root["$defs"][name];
        }

        if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
            let object_branch = branches
                .iter()
                .find(|branch| {
                    branch.get("$ref").is_some()
                        || branch.get("type").and_then(Value::as_str) == Some("object")
                })
                .expect("optional context schema must contain an object branch");
            return resolve_object_schema(root, object_branch);
        }

        schema
    }

    fn schema_tag(schema: &Value) -> Option<&str> {
        let kind = schema.get("properties")?.get("kind")?;
        kind.get("const")
            .and_then(Value::as_str)
            .or_else(|| kind.get("enum")?.as_array()?.first()?.as_str())
    }

    #[test]
    fn remember_context_maps_full_values_exactly() {
        let params: RememberParams = serde_json::from_value(json!({
            "content": "preserve context",
            "namespace": "episode",
            "kind": "decision",
            "context": {
                "product": " concord ",
                "work_id": "change-42",
                "work_kind": "change",
                "origin_repo": "episode",
                "origin_ref": "concord#46",
                "tags": ["alpha", "alpha", " beta "],
                "severity": "high"
            }
        }))
        .expect("supported context must deserialize");

        assert_eq!(params.namespace.as_deref(), Some("episode"));
        assert_eq!(params.kind.as_deref(), Some("decision"));
        assert_eq!(
            context_to_metadata(params.context).expect("context must serialize"),
            json!({
                "product": " concord ",
                "work_id": "change-42",
                "work_kind": "change",
                "origin_repo": "episode",
                "origin_ref": "concord#46",
                "tags": ["alpha", "alpha", " beta "],
                "severity": "high"
            })
        );
    }

    #[test]
    fn remember_context_maps_absent_empty_and_sparse_values() {
        let absent: RememberParams = serde_json::from_value(json!({ "content": "absent" }))
            .expect("context must remain optional");
        assert_eq!(
            context_to_metadata(absent.context).expect("absent context must serialize"),
            json!({})
        );

        let empty: RememberParams =
            serde_json::from_value(json!({ "content": "empty", "context": {} }))
                .expect("empty context must deserialize");
        assert_eq!(
            context_to_metadata(empty.context).expect("empty context must serialize"),
            json!({})
        );

        let sparse: RememberParams = serde_json::from_value(json!({
            "content": "sparse",
            "context": { "work_id": "change-7" }
        }))
        .expect("sparse context must deserialize");
        assert_eq!(
            context_to_metadata(sparse.context).expect("sparse context must serialize"),
            json!({ "work_id": "change-7" })
        );

        let explicit_null: RememberParams = serde_json::from_value(json!({
            "content": "null action",
            "context": { "action": null }
        }))
        .expect("null action must remain compatible");
        assert_eq!(
            context_to_metadata(explicit_null.context).expect("null action must serialize"),
            json!({})
        );
    }

    #[test]
    fn remember_context_maps_each_action_variant_exactly() {
        let cases = [
            (
                json!({
                    "content": "resolved",
                    "context": {
                        "action": {
                            "kind": "ad_hoc_resolved",
                            "summary": " fixed locally "
                        }
                    }
                }),
                json!({
                    "action": {
                        "kind": "ad_hoc_resolved",
                        "summary": " fixed locally "
                    }
                }),
            ),
            (
                json!({
                    "content": "linked",
                    "context": {
                        "work_id": " change-42 ",
                        "action": { "kind": "linked_work" }
                    }
                }),
                json!({
                    "work_id": " change-42 ",
                    "action": { "kind": "linked_work" }
                }),
            ),
            (
                json!({
                    "content": "open",
                    "context": { "action": { "kind": "open_followup" } }
                }),
                json!({ "action": { "kind": "open_followup" } }),
            ),
        ];

        for (input, expected) in cases {
            let params: RememberParams =
                serde_json::from_value(input).expect("valid action must deserialize");
            assert_eq!(
                context_to_metadata(params.context).expect("valid action must serialize"),
                expected
            );
        }
    }

    #[test]
    fn remember_context_rejects_malformed_action_shapes() {
        let invalid_actions = [
            json!({ "kind": "unknown" }),
            json!({ "kind": "ad_hoc_resolved" }),
            json!({ "kind": "ad_hoc_resolved", "summary": "fixed", "extra": true }),
            json!({ "kind": "linked_work", "extra": true }),
            json!({ "kind": "open_followup", "extra": true }),
        ];

        for action in invalid_actions {
            let result = serde_json::from_value::<RememberParams>(json!({
                "content": "invalid",
                "context": { "action": action }
            }));
            assert!(result.is_err(), "malformed action must reject");
        }
    }

    #[test]
    fn remember_context_rejects_invalid_action_invariants() {
        let invalid_contexts = [
            json!({ "action": { "kind": "ad_hoc_resolved", "summary": "" } }),
            json!({ "action": { "kind": "ad_hoc_resolved", "summary": "   " } }),
            json!({ "action": { "kind": "linked_work" } }),
            json!({
                "work_id": "   ",
                "action": { "kind": "linked_work" }
            }),
        ];

        for context in invalid_contexts {
            let params: RememberParams = serde_json::from_value(json!({
                "content": "invalid",
                "context": context
            }))
            .expect("structurally valid action must deserialize");
            let error = context_to_metadata(params.context)
                .expect_err("invalid action invariant must reject");
            assert_eq!(error.code, rmcp::model::ErrorCode::INVALID_PARAMS);
        }
    }

    #[test]
    fn remember_handler_preserves_invalid_params_from_context_mapper() {
        let source = include_str!("server.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("server source must contain production code");
        assert!(
            production.contains("let metadata = context_to_metadata(p.context)?;"),
            "remember must propagate the mapper's ErrorData unchanged"
        );
        assert!(
            !production.contains("context_to_metadata(p.context).map_err(internal)"),
            "remember must not recategorize invalid caller input as an internal error"
        );
    }

    #[test]
    fn recall_filters_validate_closed_optional_input() {
        let valid: RecallFilters = serde_json::from_value(json!({
            "product": "episode",
            "work_id": "change-1",
            "tags": ["memory", "recall"],
            "kinds": ["gotcha", "decision"],
            "sources": ["manual", "adv_wisdom"],
            "max_age_days": 90
        }))
        .expect("valid filters deserialize");
        assert!(!valid.include_open_followups);
        // AC3: an omitted flag must default to exclusion, so existing callers
        // stop receiving graduated knowledge without changing their request.
        assert!(!valid.include_promoted);
        valid.validate().expect("valid filters pass");

        for invalid in [
            json!({"product": " "}),
            json!({"work_id": ""}),
            json!({"tags": []}),
            json!({"tags": ["ok", " "]}),
            json!({"kinds": []}),
            json!({"kinds": [""]}),
            json!({"sources": []}),
            json!({"max_age_days": 0}),
        ] {
            let filters: RecallFilters = serde_json::from_value(invalid).unwrap();
            assert!(filters.validate().is_err());
        }
        // The sources set is closed: unknown provenance names reject at
        // deserialization, before validation runs.
        assert!(serde_json::from_value::<RecallFilters>(json!({"sources": ["concord"]})).is_err());
        assert!(serde_json::from_value::<RecallFilters>(json!({"unknown": true})).is_err());
    }

    #[test]
    fn remember_context_rejects_unknown_keys() {
        let result = serde_json::from_value::<RememberParams>(json!({
            "content": "typo",
            "context": { "workd_id": "change-42" }
        }));
        assert!(
            result.is_err(),
            "unknown context keys must reject the request"
        );
    }

    #[test]
    fn remember_context_schema_is_optional_closed_and_exact() {
        let root = serde_json::to_value(rmcp::schemars::schema_for!(RememberParams))
            .expect("remember schema must serialize");
        let required = root["required"]
            .as_array()
            .expect("remember schema must list required fields");
        assert_eq!(required, &[json!("content")]);

        let context = resolve_object_schema(&root, &root["properties"]["context"]);
        assert_eq!(context["additionalProperties"], json!(false));

        let actual = context["properties"]
            .as_object()
            .expect("context schema must expose properties")
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let expected = [
            "action",
            "origin_ref",
            "origin_repo",
            "product",
            "severity",
            "tags",
            "work_id",
            "work_kind",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);

        let action = resolve_object_schema(&root, &context["properties"]["action"]);
        let variants = action
            .get("oneOf")
            .or_else(|| action.get("anyOf"))
            .and_then(Value::as_array)
            .expect("action schema must expose variant branches");
        let actual_kinds = variants
            .iter()
            .filter_map(schema_tag)
            .collect::<BTreeSet<_>>();
        let expected_kinds = ["ad_hoc_resolved", "linked_work", "open_followup"]
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_kinds, expected_kinds);
        for variant in variants {
            assert_eq!(
                variant["additionalProperties"],
                json!(false),
                "each action variant must be closed"
            );
        }

        let empty = MemoryContext::default();
        assert_eq!(
            context_to_metadata(Some(empty)).expect("default context must serialize"),
            json!({})
        );
    }
}
