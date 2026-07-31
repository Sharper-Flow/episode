# rq-episode-operator-logging01

## Statement

Given a valid operator log-level setting, when Episode starts, then logs use that level, retain `INFO` as the default, and remain on stderr.

## Rationale

Episode uses stdio for MCP JSON-RPC. stdout is reserved for protocol messages; logs must go to stderr. Operators need a supported way to adjust verbosity, but misconfiguration must not break startup.

## Authority

- `EPISODE_LOG_LEVEL` is parsed in `src/config.rs`.
- `src/main.rs` initializes the tracing subscriber on stderr with the configured level.
- `src/lib.rs` exposes a testable initialization boundary.

## Verification

- Unit tests in `src/config.rs` prove valid values parse and invalid/absent/empty values default to `INFO`.
- No application code writes logs to stdout.

## Constraints

- stdout remains reserved for MCP protocol traffic.
