#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ -n "${RESTGAP_SKIP_PREFLIGHT:-}" ]]; then
  echo "RESTGAP_SKIP_PREFLIGHT is set; skipping release preflight."
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required but not found in PATH."
  exit 1
fi

echo "Running release preflight..."
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
