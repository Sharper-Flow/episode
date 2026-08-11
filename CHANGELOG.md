# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- `recall` supports bound product, work, tag, and kind filters backed by a JSONB GIN index, with open follow-ups excluded by default.
- `remember` accepts optional structured work context, including validated act-then-record action state.
- CI-gated automated releases triggered after a successful `main` branch CI run.
- Release profile compilation step added to CI.
- Release documentation describing versioning, changelog, and artifact conventions.

## [0.1.0] - 2026-07-30

### Added

- Initial `episode` MCP server with `recall`, `remember`, `forget`, and `stats` tools.
- Persistent decision memory backed by PostgreSQL + pgvector (HNSW cosine, `vector(1024)`).
- Local `fastembed` embedding backend (BGE-large, 1024d).
- ADV wisdom/reflection ingestion from `.adv/wisdom.jsonl` and `.adv/reflections.jsonl`.

[Unreleased]: https://github.com/Sharper-Flow/episode/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Sharper-Flow/episode/releases/tag/v0.1.0
