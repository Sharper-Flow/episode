# rq-episode-release-build01

## Statement

Given a supported pull request, when CI runs, then release-profile compilation completes successfully.

## Rationale

The release profile enables aggressive optimizations (`opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`). A debug-only CI pass can miss release-only link or codegen failures. Proving the release build in CI prevents discovering these problems at publish time.

## Authority

- `Cargo.toml` declares `[profile.release]`.
- `.github/workflows/ci.yml` runs `cargo build --release --locked`.
- `.github/workflows/release.yml` reuses the same profile to build publish artifacts.

## Verification

- CI job `Release profile` must pass on every push to `main` and every pull request.
- `scripts/deploy.sh` uses `cargo build --release --locked` and installs the resulting binary to `~/.local/bin/episode`.

## Constraints

- Release publishing must occur only after CI succeeds on `main`.
