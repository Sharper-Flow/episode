//! Parse ADV wisdom/reflection JSONL into embeddable memory inputs.
//!
//! These are PURE functions (no DB, no embeddings). The ingestion loop that
//! calls them, dedups, embeds, and upserts lives in `main.rs`.

use anyhow::Result;
use std::path::Path;

use crate::types::MemoryInput;

/// WORKER C — parse `{adv_dir}/wisdom.jsonl`.
///
/// Each line is a JSON object (ADV `ProjectWisdomEntry`):
///   `{ id: "pw-..", type, content, source_change?, source_task?, promoted_at, tags?, ... }`
/// Map each ->
///   `MemoryInput { id: <id>, namespace, source: MemorySource::AdvWisdom,
///     source_id: Some(<id>), kind: Some(<type>), content: <content>,
///     metadata: <the raw object with `content` removed> }`.
/// Rules:
///   - skip blank/malformed lines (do NOT fail the whole file — mirror ADV's
///     `parseWisdomEntries` graceful-degradation behavior).
///   - if the file does not exist, return `Ok(vec![])`.
pub fn parse_wisdom(_namespace: &str, _adv_dir: &Path) -> Result<Vec<MemoryInput>> {
    todo!("WORKER C: parse wisdom.jsonl")
}

/// WORKER C — parse `{adv_dir}/reflections.jsonl`.
///
/// Each line is an ADV `ReflectionEntry`:
///   `{ id: "rf-..", change_id, created_at, plane1{..}, plane2{ friction_items[],
///     highlights[], improvement_suggestions[] } }`.
/// Explode each reflection into MULTIPLE `MemoryInput`s (the retrievable text
/// lives in plane2):
///   - one per `plane2.friction_items[i].description`  -> kind "friction"
///   - one per `plane2.highlights[i]`                  -> kind "highlight"
///   - one per `plane2.improvement_suggestions[i]`     -> kind "suggestion"
/// For each child:
///   - stable id / source_id: `format!("{}:{}:{}", rf_id, kind, index)`
///   - source: `MemorySource::AdvReflection`
///   - content: the text
///   - metadata: `{ "change_id": .., "reflection_id": rf_id, "created_at": .. }`
/// Rules: skip malformed lines; missing file -> `Ok(vec![])`.
pub fn parse_reflections(_namespace: &str, _adv_dir: &Path) -> Result<Vec<MemoryInput>> {
    todo!("WORKER C: parse reflections.jsonl")
}
