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

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required but not found in PATH."
  exit 1
fi

host_target="$(rustc -vV | awk '/host:/ {print $2}')"

resolve_extra_targets() {
  if [[ -n "${RESTGAP_EXTRA_TARGETS:-}" ]]; then
    printf '%s\n' ${RESTGAP_EXTRA_TARGETS}
    return
  fi

  rustup target list --installed | while read -r target; do
    if [[ "${target}" != "${host_target}" ]]; then
      printf '%s\n' "${target}"
    fi
  done
}

echo "Running release preflight..."
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings

while read -r target; do
  if [[ -z "${target}" ]]; then
    continue
  fi
  echo "Running cross-target checks for ${target}..."
  cargo clippy --target "${target}" --all-targets --all-features -- -D warnings
  cargo check --target "${target}" --all-targets
done < <(resolve_extra_targets)

cargo test
cargo build --release
