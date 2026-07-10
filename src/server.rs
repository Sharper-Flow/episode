//! rmcp server exposing recall / remember / forget / stats tools.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, ServerCapabilities, ServerInfo};
use rmcp::schemars::{self, JsonSchema};
use rmcp::{tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::Deserialize;

use crate::embed::Embedder;
use crate::store::Store;
use crate::types::{MemoryInput, MemorySource};

#[derive(Clone)]
pub struct EpisodeServer {
    store: Store,
    embedder: Arc<dyn Embedder>,
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
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ForgetParams {
    /// The memory id to remove.
    id: String,
}

fn internal(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}

#[tool_router]
impl EpisodeServer {
    pub fn new(store: Store, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            store,
            embedder,
            tool_router: Self::tool_router(),
        }
    }

    async fn embed_query(&self, text: String) -> Result<Vec<f32>, ErrorData> {
        let embedder = self.embedder.clone();
        tokio::task::spawn_blocking(move || embedder.embed_one(&text))
            .await
            .map_err(internal)?
            .map_err(internal)
    }

    #[tool(
        description = "Semantically recall durable decision memories (gotchas, conventions, failed approaches, reflections) relevant to a query. Namespace-scoped; results include a similarity score."
    )]
    async fn recall(
        &self,
        Parameters(p): Parameters<RecallParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let embedding = self.embed_query(p.query).await?;
        let namespaces = match p.namespace {
            Some(ns) => vec![ns, "global".to_string()],
            None => Vec::new(),
        };
        let top_k = p.top_k.unwrap_or(8).clamp(1, 50);
        let hits = self
            .store
            .recall(&embedding, &namespaces, top_k)
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
        let embedding = self.embed_query(p.content.clone()).await?;
        let id = format!("mem-{}", uuid::Uuid::new_v4().simple());
        let input = MemoryInput {
            id: id.clone(),
            namespace: p.namespace.unwrap_or_else(|| "global".to_string()),
            source: MemorySource::Manual,
            source_id: None,
            kind: p.kind,
            content: p.content,
            metadata: serde_json::json!({}),
        };
        self.store.upsert(&input, &embedding).await.map_err(internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::json(
            &serde_json::json!({ "stored": true, "id": id }),
        )?]))
    }

    #[tool(description = "Remove a memory by id.")]
    async fn forget(
        &self,
        Parameters(p): Parameters<ForgetParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let removed = self.store.forget(&p.id).await.map_err(internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::json(
            &serde_json::json!({ "removed": removed }),
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
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "episode: persistent decision memory for agents. Use `recall` before starting \
             work to surface prior gotchas/conventions/decisions; use `remember` to store \
             durable learnings. Namespaces are per-project plus a shared `global`.",
        )
    }
}
