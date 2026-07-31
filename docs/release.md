# Releases

Episode releases are automated and CI-gated. Nothing is published from a pull
request or from a manual tag push; the release workflow runs only after the `CI`
workflow succeeds on the `main` branch.

## How releases work

1. A contributor opens a PR with the changes they want to ship.
2. `CI` runs on that PR: format, clippy, debug tests, **release-profile build**,
   and dependency advisories.
3. After the PR merges to `main`, `CI` runs again on `main`.
4. If `CI` succeeds, the `Release` workflow is triggered automatically.
5. The release workflow reads the version from `Cargo.toml`, builds the release
   binary, and publishes a GitHub release with a matching `v{version}` tag and
   artifacts.

A release is only created when a tag with the current `Cargo.toml` version
does not already exist. Re-running CI for an already-released version is a safe
no-op.

## Preparing a release

1. Update the version in `Cargo.toml` following [Semantic Versioning](https://semver.org/).
2. Add a new section to `CHANGELOG.md` for the new version, move notable items
   from `[Unreleased]`, and update the comparison links at the bottom of the
   file.
3. Open a PR and get it merged to `main`.
4. Once `CI` passes, the release workflow creates:
   - a Git tag `v{version}` pointing at the merged commit,
   - a GitHub release with auto-generated release notes,
   - the attached artifacts listed below.

## Release artifacts

| Artifact | Description |
|---|---|
| `episode` | Optimized Linux x86_64 binary built with `cargo build --release --locked` using the `[profile.release]` settings in `Cargo.toml`. |
| `episode.sha256` | `sha256sum` of the `episode` binary. |
| `CHANGELOG.md` | Human-readable changelog for the release. |
| `LICENSE` | Project license. |

The release binary is built with the release profile documented in `Cargo.toml`:
`opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, and `strip = "symbols"`.

## Workflow permissions

The release job uses the smallest permission set required to create a release and
tag:

```yaml
permissions:
  contents: write
```

No other scopes are granted. The workflow is never triggered by PR events and
never publishes from a branch other than `main`.

## Related files

- `.github/workflows/ci.yml` — CI definition, including the release-profile build.
- `.github/workflows/release.yml` — automated release definition.
- `Cargo.toml` — package version and release profile.
- `CHANGELOG.md` — release notes source.
- `scripts/deploy.sh` — local manual deployment helper (separate from automated
  GitHub releases).
