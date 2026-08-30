//! Parse ADV wisdom/reflection JSONL into embeddable memory inputs.
//!
//! These are PURE functions (no DB, no embeddings). The ingestion loop that
//! calls them, dedups, embeds, and upserts lives in `main.rs`.
//!
//! Parsing is deliberately lenient: ADV schemas carry many optional fields and
//! evolve over time, so we work with `serde_json::Value`, skip blank/malformed
//! lines, and never fail the whole file because one line is bad. A missing
//! target file is treated as an empty source (`Ok(vec![])`).

use anyhow::Result;
use serde_json::Value;
use std::path::Path;

use crate::types::{MemoryInput, MemorySource, PROMOTION_STATE_KEY};

/// Return non-empty string slices out of a JSON value, else `None`.
fn nonempty_str(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty())
}

/// Read a JSONL file into raw `Value` lines, skipping blank/malformed lines.
/// Returns `Ok(vec![])` when the file does not exist.
fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) if v.is_object() => out.push(v),
            _ => continue, // malformed or non-object line: skip gracefully
        }
    }
    Ok(out)
}

/// Outcome of parsing `wisdom.jsonl`: rows to store, plus state changes that
/// apply to rows already stored.
///
/// The second half exists because ingestion is write-once per `source_id`.
/// Dedup drops an already-stored item before it reaches the store, so an ADV
/// field that changes *after* first ingest can only be applied by addressing the
/// stored row directly.
#[derive(Debug, Default)]
pub struct WisdomParse {
    /// Entries eligible to be embedded and stored.
    pub items: Vec<MemoryInput>,
    /// Source ids whose ADV entry now carries a non-null `invalidated_by`.
    pub invalidated: Vec<String>,
    /// Source ids whose ADV entry now carries a non-null `promoted_at`.
    ///
    /// Collected from the file on every pass, so a failed update simply retries
    /// on the next reconcile.
    pub promoted: Vec<String>,
}

/// Parse `{adv_dir}/wisdom.jsonl` into memory inputs.
///
/// Each line is a JSON object (ADV `ProjectWisdomEntry`):
///   `{ id: "pw-..", type, content, source_change?, source_task?, promoted_at, tags?, ... }`
/// Map each ->
///   `MemoryInput { id: <id>, namespace, source: MemorySource::AdvWisdom,
///     source_id: Some(<id>), kind: Some(<type>), content: <content>,
///     metadata: <the raw object minus the keys episode owns> }`.
/// Rules:
///   - skip blank/malformed lines (do NOT fail the whole file — mirror ADV's
///     `parseWisdomEntries` graceful-degradation behavior).
///   - skip entries whose `invalidated_by` is present and non-null
///     (superseded / soft-deleted), and collect their ids so reconcile can
///     remove copies stored before the retraction.
///   - if the file does not exist, return an empty [`WisdomParse`].
pub fn parse_wisdom(namespace: &str, adv_dir: &Path) -> Result<WisdomParse> {
    let path = adv_dir.join("wisdom.jsonl");
    let mut items = Vec::new();
    let mut invalidated = Vec::new();
    let mut promoted = Vec::new();

    for value in read_jsonl(&path)? {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        let id = match obj.get("id").and_then(nonempty_str) {
            Some(s) => s.to_string(),
            None => continue,
        };

        // Superseded / soft-deleted entries are not retrievable. Skipping keeps
        // them out; collecting the id is what removes an already-stored copy.
        if obj.get("invalidated_by").is_some_and(|v| !v.is_null()) {
            invalidated.push(id);
            continue;
        }

        let content = match obj.get("content").and_then(nonempty_str) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let kind = obj
            .get("type")
            .and_then(nonempty_str)
            .map(|s| s.to_string());

        // ADV records graduation as a timestamp. Episode records it as state,
        // applied to the stored row by reconcile. Collecting the id here rather
        // than writing the state inline keeps one writer for `promotion_state`:
        // an entry promoted before first ingest and one promoted after it take
        // the same path.
        if obj.get("promoted_at").is_some_and(|v| !v.is_null()) {
            promoted.push(id.clone());
        }

        // Metadata = full object minus the keys episode owns. `content` is the
        // embeddable text; everything else is provenance/filter data.
        //
        // `promoted_at` goes so `promotion_state` is the only promotion field
        // agents can filter on. Keeping both would give them a second field
        // populated only on entries that arrived already-promoted, since dedup
        // freezes metadata at first write.
        //
        // `promotion_state` goes because this object is untrusted input and the
        // key is episode's. Reconcile is its only writer. Left in place, a
        // source entry could forge `promoted` and hide itself from recall on
        // first ingest, or write a malformed value that no transition can
        // address.
        let mut metadata = value.clone();
        if let Some(map) = metadata.as_object_mut() {
            map.remove("content");
            map.remove("promoted_at");
            map.remove(PROMOTION_STATE_KEY);
        }

        items.push(MemoryInput {
            source_id: Some(id.clone()),
            id,
            namespace: namespace.to_string(),
            source: MemorySource::AdvWisdom,
            kind,
            content,
            metadata,
        });
    }

    Ok(WisdomParse {
        items,
        invalidated,
        promoted,
    })
}

/// Parse `{adv_dir}/reflections.jsonl` into memory inputs.
///
/// Each line is an ADV `ReflectionEntry`. The retrievable text lives in
/// `plane2`, so each reflection is exploded into multiple `MemoryInput`s:
///
/// - one per `plane2.friction_items[i].description` (kind `friction`)
/// - one per `plane2.highlights[i]` (kind `highlight`)
/// - one per `plane2.improvement_suggestions[i]` (kind `suggestion`)
///
/// Each child gets a stable id `format!("{rf_id}:{kind}:{index}")`,
/// `source = MemorySource::AdvReflection`, the text as `content`, and metadata
/// `{ change_id, reflection_id, created_at }`.
///
/// Malformed lines and empty/whitespace children are skipped; a missing file
/// yields `Ok(vec![])`.
pub fn parse_reflections(namespace: &str, adv_dir: &Path) -> Result<Vec<MemoryInput>> {
    let path = adv_dir.join("reflections.jsonl");
    let mut items = Vec::new();

    for value in read_jsonl(&path)? {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        // Without a usable reflection id we cannot form stable child ids.
        let rf_id = match obj.get("id").and_then(nonempty_str) {
            Some(s) => s.to_string(),
            None => continue,
        };

        let change_id = obj.get("change_id").cloned().unwrap_or(Value::Null);
        let created_at = obj.get("created_at").cloned().unwrap_or(Value::Null);
        let metadata = serde_json::json!({
            "reflection_id": rf_id,
            "change_id": change_id,
            "created_at": created_at,
        });

        let plane2 = match obj.get("plane2").and_then(|v| v.as_object()) {
            Some(p) => p,
            None => continue,
        };

        // friction_items: objects with a `description` field.
        if let Some(arr) = plane2.get("friction_items").and_then(|v| v.as_array()) {
            for (i, item) in arr.iter().enumerate() {
                let desc = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                push_reflection_child(
                    &mut items, &rf_id, namespace, "friction", i, desc, &metadata,
                );
            }
        }
        // highlights: plain strings.
        if let Some(arr) = plane2.get("highlights").and_then(|v| v.as_array()) {
            for (i, item) in arr.iter().enumerate() {
                if let Some(s) = item.as_str() {
                    push_reflection_child(
                        &mut items,
                        &rf_id,
                        namespace,
                        "highlight",
                        i,
                        s,
                        &metadata,
                    );
                }
            }
        }
        // improvement_suggestions: plain strings.
        if let Some(arr) = plane2
            .get("improvement_suggestions")
            .and_then(|v| v.as_array())
        {
            for (i, item) in arr.iter().enumerate() {
                if let Some(s) = item.as_str() {
                    push_reflection_child(
                        &mut items,
                        &rf_id,
                        namespace,
                        "suggestion",
                        i,
                        s,
                        &metadata,
                    );
                }
            }
        }
    }

    Ok(items)
}

/// Push one exploded reflection child, skipping empty/whitespace content.
#[allow(clippy::too_many_arguments)]
fn push_reflection_child(
    items: &mut Vec<MemoryInput>,
    rf_id: &str,
    namespace: &str,
    kind: &str,
    index: usize,
    content: &str,
    metadata: &Value,
) {
    if content.trim().is_empty() {
        return;
    }
    let id = format!("{}:{}:{}", rf_id, kind, index);
    items.push(MemoryInput {
        source_id: Some(id.clone()),
        id,
        namespace: namespace.to_string(),
        source: MemorySource::AdvReflection,
        kind: Some(kind.to_string()),
        content: content.to_string(),
        metadata: metadata.clone(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Unique temp dir that cleans itself up on drop (no `tempfile` crate).
    struct TmpDir(PathBuf);

    impl TmpDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "episode-ingest-{}-{}-{}",
                label,
                std::process::id(),
                nanos
            ));
            std::fs::create_dir_all(&path).unwrap();
            TmpDir(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn parse_wisdom_skips_malformed_and_invalidated() {
        let tmp = TmpDir::new("wisdom");
        let adv = tmp.path();
        std::fs::write(
            adv.join("wisdom.jsonl"),
            [
                r#"{"id":"pw-1","type":"gotcha","content":"use X not Y","source_change":"c1","tags":["a","b"]}"#,
                "this is not json at all",
                r#"{"id":"pw-2","type":"pattern","content":"valid two","promoted_at":"2026-07-07T02:13:34.418Z"}"#,
                r#"{"id":"pw-3","type":"gotcha","content":"superseded","invalidated_by":"pw-9"}"#,
                r#"{"id":"pw-4","type":"gotcha","content":"null-invalidation-kept","invalidated_by":null}"#,
                r#"{"id":"pw-5","type":"gotcha","content":"forged state","promotion_state":{"kind":"promoted","target":"forged"}}"#,
                "",
            ]
            .join("\n"),
        )
        .unwrap();

        let parsed = parse_wisdom("proj", adv).unwrap();
        let got = &parsed.items;

        // pw-3 dropped (invalidated_by non-null); malformed line dropped;
        // blank dropped; pw-4 and pw-5 kept (pw-4's invalidated_by is null) => 4 kept.
        assert_eq!(got.len(), 4);

        // Skipping keeps an invalidated entry out; the collected id is what lets
        // reconcile remove one that was already stored before the retraction.
        assert_eq!(parsed.invalidated, vec!["pw-3".to_string()]);

        // pw-2 carries promoted_at. Its id is collected so reconcile can apply the
        // state to a row that dedup would otherwise skip, and the raw timestamp is
        // stripped so `promotion_state` is the only promotion field agents see.
        assert_eq!(parsed.promoted, vec!["pw-2".to_string()]);
        assert_eq!(got[1].id, "pw-2");
        assert!(
            got[1].metadata.get("promoted_at").is_none(),
            "promoted_at must not persist alongside promotion_state"
        );

        // pw-5 forges episode's reserved key. Reconcile is its only writer, so
        // the source-supplied value must be dropped rather than stored.
        assert_eq!(got[3].id, "pw-5");
        assert!(
            got[3].metadata.get(PROMOTION_STATE_KEY).is_none(),
            "a source-supplied promotion_state must never reach the store"
        );
        assert_eq!(
            got[3].metadata.get("type").and_then(|v| v.as_str()),
            Some("gotcha"),
            "stripping the reserved key must not disturb real provenance"
        );

        let first = &got[0];
        assert_eq!(first.id, "pw-1");
        assert_eq!(first.source_id.as_deref(), Some("pw-1"));
        assert_eq!(first.namespace, "proj");
        assert_eq!(first.source, MemorySource::AdvWisdom);
        assert_eq!(first.kind.as_deref(), Some("gotcha"));
        assert_eq!(first.content, "use X not Y");
        // metadata keeps provenance, drops the embeddable `content`.
        assert!(first.metadata.get("content").is_none());
        assert_eq!(
            first.metadata.get("type").and_then(|v| v.as_str()),
            Some("gotcha")
        );
        assert_eq!(
            first.metadata.get("source_change").and_then(|v| v.as_str()),
            Some("c1")
        );
        assert_eq!(
            first
                .metadata
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|a| a.len()),
            Some(2)
        );

        assert_eq!(got[2].id, "pw-4");
    }

    #[test]
    fn parse_reflections_explodes_plane2_children() {
        let tmp = TmpDir::new("reflect");
        let adv = tmp.path();
        std::fs::write(
            adv.join("reflections.jsonl"),
            [
                r#"{"id":"rf-xyz","change_id":"chg1","created_at":"2026-07-07T00:00:00Z","plane2":{"friction_items":[{"category":"tool_gap","description":"X was hard","tool_name":"foo"}],"highlights":["shipped Y","   "],"improvement_suggestions":["do Z"]}}"#,
                "garbage line",
                r#"{"id":"rf-empty","plane2":{"friction_items":[],"highlights":[],"improvement_suggestions":[]}}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let got = parse_reflections("proj", adv).unwrap();

        // rf-xyz: 1 friction + 2 highlights (one whitespace -> skipped) + 1 suggestion = 3.
        // rf-empty contributes 0. garbage line skipped.
        assert_eq!(got.len(), 3);

        let f = &got[0];
        assert_eq!(f.id, "rf-xyz:friction:0");
        assert_eq!(f.source_id.as_deref(), Some("rf-xyz:friction:0"));
        assert_eq!(f.source, MemorySource::AdvReflection);
        assert_eq!(f.kind.as_deref(), Some("friction"));
        assert_eq!(f.content, "X was hard");
        assert_eq!(f.namespace, "proj");
        assert_eq!(
            f.metadata.get("reflection_id").and_then(|v| v.as_str()),
            Some("rf-xyz")
        );
        assert_eq!(
            f.metadata.get("change_id").and_then(|v| v.as_str()),
            Some("chg1")
        );
        assert_eq!(
            f.metadata.get("created_at").and_then(|v| v.as_str()),
            Some("2026-07-07T00:00:00Z")
        );

        assert_eq!(got[1].id, "rf-xyz:highlight:0");
        assert_eq!(got[1].content, "shipped Y");
        assert_eq!(got[2].id, "rf-xyz:suggestion:0");
        assert_eq!(got[2].content, "do Z");
    }

    #[test]
    fn missing_files_return_empty() {
        let tmp = TmpDir::new("missing");
        let adv = tmp.path(); // empty dir: no wisdom.jsonl / reflections.jsonl
        let parsed = parse_wisdom("proj", adv).unwrap();
        assert!(parsed.items.is_empty());
        assert!(parsed.invalidated.is_empty());
        assert!(parse_reflections("proj", adv).unwrap().is_empty());
    }
}
