#!/usr/bin/env bash
# Build episode in release and deploy the binary for Vision to spawn.
# Vision's servers.yaml points at ~/.local/bin/episode (see repo README).
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> building release"
cargo build --release

dest="${HOME}/.local/bin/episode"
install -Dm755 target/release/episode "$dest"
echo "==> deployed: $dest"

cat <<'NOTE'
==> next steps
  - ensure the dev DB is up:   docker compose up -d
  - restart Vision to pick up a new binary:
      systemctl --user restart vision.service
NOTE
