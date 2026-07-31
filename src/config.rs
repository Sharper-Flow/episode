//! Environment-driven configuration.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing_subscriber::filter::LevelFilter;

/// A project root to ingest ADV wisdom/reflections from, with its namespace.
#[derive(Debug, Clone)]
pub struct ProjectRoot {
    pub namespace: String,
    pub path: PathBuf,
}

/// Embedding backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedBackend {
    /// Local fastembed (default, and the only backend supported in v0).
    Local,
    /// Voyage API (`voyage-4-lite`). Parsed as a known variant but rejected
    /// during validation as unsupported in v0 (see `Config::from_vars`).
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
    /// Maximum log level emitted to stderr (stdout is reserved for MCP JSON-RPC).
    /// Absent, empty, or invalid values deterministically default to INFO.
    pub log_level: LevelFilter,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_vars(std::env::vars())
    }

    /// Parse configuration from an explicit `(key, value)` source, validating
    /// every field deterministically.
    ///
    /// Internal seam so validation can be unit-tested without mutating the
    /// process environment (Rust 2024 marks `set_var`/`remove_var` unsafe).
    /// `from_env` delegates here.
    ///
    /// Validation policy (AC2 / DONT2):
    ///   - absent optional vars use their documented defaults;
    ///   - a *present* but invalid/zero pool size or ingest interval errors,
    ///     naming the variable and the expected form;
    ///   - `EPISODE_EMBED_BACKEND` accepts only `local`; `voyage` and unknown
    ///     values error rather than silently falling back;
    ///   - every `EPISODE_PROJECT_ROOTS` entry must be a non-empty
    ///     `namespace=path`; malformed or empty entries error with the entry
    ///     text.
    fn from_vars<I: IntoIterator<Item = (String, String)>>(vars: I) -> Result<Self> {
        let map: HashMap<String, String> = vars.into_iter().collect();
        let get = |k: &str| -> Option<String> { map.get(k).cloned() };

        let database_url =
            get("EPISODE_DATABASE_URL").context("EPISODE_DATABASE_URL must be set")?;

        let pool_size = parse_nonzero(get("EPISODE_DB_POOL_SIZE"), "EPISODE_DB_POOL_SIZE", 10)?;
        let ingest_interval_secs = parse_nonzero(
            get("EPISODE_INGEST_INTERVAL_SECS"),
            "EPISODE_INGEST_INTERVAL_SECS",
            60,
        )?;

        let embed_backend = parse_backend(get("EPISODE_EMBED_BACKEND"))?;
        let voyage_api_key = get("VOYAGE_API_KEY");
        let project_roots = parse_project_roots(get("EPISODE_PROJECT_ROOTS"))?;
        let log_level = parse_log_level(get("EPISODE_LOG_LEVEL"));

        Ok(Self {
            database_url,
            pool_size,
            embed_backend,
            voyage_api_key,
            project_roots,
            ingest_interval_secs,
            log_level,
        })
    }
}

/// Parse the `EPISODE_LOG_LEVEL` variable. Absent, empty, or invalid values
/// silently fall back to INFO so a logging misconfiguration cannot prevent
/// startup (C4 / stderr safety is enforced by the binary subscriber setup).
fn parse_log_level(raw: Option<String>) -> LevelFilter {
    let Some(raw) = raw else {
        return LevelFilter::INFO;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return LevelFilter::INFO;
    }
    trimmed.parse::<LevelFilter>().unwrap_or(LevelFilter::INFO)
}

/// Parse a present-then-strict non-zero positive integer, or return the
/// documented default when the variable is absent. A present value that is
/// non-numeric, out of range, or zero errors with the variable name, the
/// offending value, and the expected form.
fn parse_nonzero<T>(raw: Option<String>, name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr + PartialOrd + From<u8> + std::fmt::Display,
    T::Err: std::fmt::Display,
{
    let Some(raw) = raw else {
        return Ok(default);
    };
    let value: T = raw.trim().parse().map_err(|e| {
        anyhow::anyhow!("{name} must be a non-zero positive integer (got {raw:?}): {e}")
    })?;
    if value == T::from(0u8) {
        anyhow::bail!("{name} must be a non-zero positive integer (got {raw:?})");
    }
    Ok(value)
}

/// Parse the embedding backend. Only `local` is supported in v0. Absent (or
/// unset) yields the documented `local` default; `voyage` errors as unsupported
/// (fail at validation instead of starting up and aborting later); any other
/// value errors as unknown.
fn parse_backend(raw: Option<String>) -> Result<EmbedBackend> {
    match raw
        .as_deref()
        .map(str::trim)
        .map(|s| s.to_ascii_lowercase())
    {
        None => Ok(EmbedBackend::Local),
        Some(s) if s == "local" => Ok(EmbedBackend::Local),
        Some(s) if s == "voyage" => anyhow::bail!(
            "EPISODE_EMBED_BACKEND value \"voyage\" is unsupported; only \"local\" is supported in v0"
        ),
        Some(other) => {
            anyhow::bail!("EPISODE_EMBED_BACKEND has unknown value {other:?}; expected \"local\"")
        }
    }
}

/// Parse `EPISODE_PROJECT_ROOTS` into validated project roots. An absent or
/// wholly-empty value yields no roots (no ingestion). Within a non-empty value,
/// every comma-separated entry must be a non-empty `namespace=path`; an empty
/// segment (consecutive/trailing comma) or a malformed entry errors, naming the
/// variable and the offending entry.
fn parse_project_roots(raw: Option<String>) -> Result<Vec<ProjectRoot>> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    let mut roots = Vec::new();
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            anyhow::bail!(
                "EPISODE_PROJECT_ROOTS contains an empty entry; expected non-empty namespace=path entries"
            );
        }
        let (ns, path) = entry.split_once('=').ok_or_else(|| {
            anyhow::anyhow!(
                "malformed EPISODE_PROJECT_ROOTS entry {entry:?}: expected non-empty namespace=path"
            )
        })?;
        let ns = ns.trim();
        let path = path.trim();
        if ns.is_empty() {
            anyhow::bail!(
                "malformed EPISODE_PROJECT_ROOTS entry {entry:?}: namespace must be non-empty (expected namespace=path)"
            );
        }
        if path.is_empty() {
            anyhow::bail!(
                "malformed EPISODE_PROJECT_ROOTS entry {entry:?}: path must be non-empty (expected namespace=path)"
            );
        }
        roots.push(ProjectRoot {
            namespace: ns.to_string(),
            path: PathBuf::from(path),
        });
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DB: &str = "postgres://episode:episode@localhost:5434/episode";

    /// Minimal fully-valid variable set. Tests clone and override the field
    /// under scrutiny so every case exercises exactly one validation rule.
    fn base() -> Vec<(String, String)> {
        vec![
            ("EPISODE_DATABASE_URL".into(), DB.into()),
            ("EPISODE_DB_POOL_SIZE".into(), "8".into()),
            ("EPISODE_INGEST_INTERVAL_SECS".into(), "30".into()),
            ("EPISODE_LOG_LEVEL".into(), "info".into()),
            ("EPISODE_EMBED_BACKEND".into(), "local".into()),
            (
                "EPISODE_PROJECT_ROOTS".into(),
                "advance=/home/jon/dev/advance".into(),
            ),
        ]
    }

    fn set(vars: &mut Vec<(String, String)>, key: &str, val: &str) {
        if let Some(slot) = vars.iter_mut().find(|(k, _)| k == key) {
            slot.1 = val.into();
        } else {
            vars.push((key.into(), val.into()));
        }
    }

    #[test]
    fn valid_config_parses() {
        let cfg = Config::from_vars(base()).expect("valid config should parse");
        assert_eq!(cfg.pool_size, 8);
        assert_eq!(cfg.ingest_interval_secs, 30);
        assert_eq!(cfg.embed_backend, EmbedBackend::Local);
        assert_eq!(cfg.project_roots.len(), 1);
        assert_eq!(cfg.project_roots[0].namespace, "advance");
    }

    #[test]
    fn documented_defaults_apply_when_absent() {
        // Only the database URL is required; everything else has a documented
        // default (these are defaults for *absent* values, not silent fallbacks
        // for invalid ones).
        let cfg = Config::from_vars(vec![("EPISODE_DATABASE_URL".into(), DB.into())])
            .expect("absent optional vars use documented defaults");
        assert_eq!(cfg.pool_size, 10);
        assert_eq!(cfg.ingest_interval_secs, 60);
        assert_eq!(cfg.embed_backend, EmbedBackend::Local);
        assert!(cfg.project_roots.is_empty());
    }

    #[test]
    fn missing_database_url_errors() {
        let err = Config::from_vars(vec![]).expect_err("database URL is required");
        assert!(
            err.to_string().contains("EPISODE_DATABASE_URL"),
            "error names the missing variable: {err}"
        );
    }

    #[test]
    fn zero_pool_size_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_DB_POOL_SIZE", "0");
        let err = Config::from_vars(v).expect_err("zero pool size must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("EPISODE_DB_POOL_SIZE"),
            "names variable: {msg}"
        );
    }

    #[test]
    fn invalid_pool_size_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_DB_POOL_SIZE", "abc");
        let err = Config::from_vars(v).expect_err("non-numeric pool size must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("EPISODE_DB_POOL_SIZE"),
            "names variable: {msg}"
        );
        assert!(msg.contains("abc"), "echoes offending value: {msg}");
    }

    #[test]
    fn zero_ingest_interval_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_INGEST_INTERVAL_SECS", "0");
        let err = Config::from_vars(v).expect_err("zero ingest interval must fail");
        assert!(err.to_string().contains("EPISODE_INGEST_INTERVAL_SECS"));
    }

    #[test]
    fn invalid_ingest_interval_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_INGEST_INTERVAL_SECS", "soon");
        let err = Config::from_vars(v).expect_err("non-numeric interval must fail");
        let msg = err.to_string();
        assert!(msg.contains("EPISODE_INGEST_INTERVAL_SECS"));
        assert!(msg.contains("soon"), "echoes offending value: {msg}");
    }

    #[test]
    fn unknown_backend_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_EMBED_BACKEND", "banana");
        let err = Config::from_vars(v).expect_err("unknown backend must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("EPISODE_EMBED_BACKEND"),
            "names variable: {msg}"
        );
        assert!(msg.contains("banana"), "echoes offending value: {msg}");
    }

    #[test]
    fn voyage_backend_rejected_as_unsupported() {
        // v0 supports only `local`; `voyage` must fail at validation rather than
        // starting up and aborting later.
        let mut v = base();
        set(&mut v, "EPISODE_EMBED_BACKEND", "voyage");
        let err = Config::from_vars(v).expect_err("voyage is unsupported in v0");
        let msg = err.to_string();
        assert!(
            msg.contains("EPISODE_EMBED_BACKEND"),
            "names variable: {msg}"
        );
        assert!(msg.contains("voyage"), "echoes offending value: {msg}");
    }

    #[test]
    fn backend_matching_is_case_insensitive_for_local() {
        let mut v = base();
        set(&mut v, "EPISODE_EMBED_BACKEND", "LOCAL");
        let cfg = Config::from_vars(v).expect("LOCAL should normalize to local");
        assert_eq!(cfg.embed_backend, EmbedBackend::Local);
    }

    #[test]
    fn malformed_root_missing_equals_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_PROJECT_ROOTS", "advance-no-equals");
        let err = Config::from_vars(v).expect_err("entry without '=' must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("EPISODE_PROJECT_ROOTS"),
            "names variable: {msg}"
        );
        assert!(
            msg.contains("advance-no-equals"),
            "echoes offending entry: {msg}"
        );
    }

    #[test]
    fn empty_root_entry_rejected() {
        // Trailing comma produces an empty segment.
        let mut v = base();
        set(&mut v, "EPISODE_PROJECT_ROOTS", "advance=/x,");
        let err = Config::from_vars(v).expect_err("empty entry must fail");
        assert!(err.to_string().contains("EPISODE_PROJECT_ROOTS"));
    }

    #[test]
    fn empty_namespace_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_PROJECT_ROOTS", "=/home/jon/dev/advance");
        let err = Config::from_vars(v).expect_err("empty namespace must fail");
        assert!(err.to_string().contains("EPISODE_PROJECT_ROOTS"));
    }

    #[test]
    fn empty_path_rejected() {
        let mut v = base();
        set(&mut v, "EPISODE_PROJECT_ROOTS", "advance=");
        let err = Config::from_vars(v).expect_err("empty path must fail");
        assert!(err.to_string().contains("EPISODE_PROJECT_ROOTS"));
    }

    #[test]
    fn log_level_defaults_to_info_when_absent() {
        let mut v = base();
        v.retain(|(k, _)| k != "EPISODE_LOG_LEVEL");
        let cfg = Config::from_vars(v).expect("valid config should parse");
        assert_eq!(cfg.log_level, tracing_subscriber::filter::LevelFilter::INFO);
    }

    #[test]
    fn log_level_defaults_to_info_when_invalid() {
        let mut v = base();
        set(&mut v, "EPISODE_LOG_LEVEL", "verbose");
        let cfg = Config::from_vars(v).expect("invalid log level must not fail startup");
        assert_eq!(cfg.log_level, tracing_subscriber::filter::LevelFilter::INFO);
    }

    #[test]
    fn log_level_defaults_to_info_when_empty_or_whitespace() {
        let mut v = base();
        set(&mut v, "EPISODE_LOG_LEVEL", "   ");
        let cfg = Config::from_vars(v).expect("whitespace-only log level must not fail startup");
        assert_eq!(cfg.log_level, tracing_subscriber::filter::LevelFilter::INFO);

        let mut v = base();
        set(&mut v, "EPISODE_LOG_LEVEL", "");
        let cfg = Config::from_vars(v).expect("empty log level must not fail startup");
        assert_eq!(cfg.log_level, tracing_subscriber::filter::LevelFilter::INFO);
    }

    #[test]
    fn log_level_valid_values_are_case_insensitive() {
        for (raw, expected) in [
            ("trace", tracing_subscriber::filter::LevelFilter::TRACE),
            ("TRACE", tracing_subscriber::filter::LevelFilter::TRACE),
            ("debug", tracing_subscriber::filter::LevelFilter::DEBUG),
            ("DEBUG", tracing_subscriber::filter::LevelFilter::DEBUG),
            ("info", tracing_subscriber::filter::LevelFilter::INFO),
            ("INFO", tracing_subscriber::filter::LevelFilter::INFO),
            ("warn", tracing_subscriber::filter::LevelFilter::WARN),
            ("WARN", tracing_subscriber::filter::LevelFilter::WARN),
            ("error", tracing_subscriber::filter::LevelFilter::ERROR),
            ("ERROR", tracing_subscriber::filter::LevelFilter::ERROR),
        ] {
            let mut v = base();
            set(&mut v, "EPISODE_LOG_LEVEL", raw);
            let cfg = Config::from_vars(v).expect("valid log level should parse");
            assert_eq!(
                cfg.log_level, expected,
                "{raw:?} should parse to {expected:?}"
            );
        }
    }
}
