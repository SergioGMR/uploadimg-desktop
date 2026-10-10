#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [[ "$(uname -s)" != Linux ]]; then
  echo "GLib regression requires Linux and existing Rust/GLib development dependencies." >&2
  exit 1
fi
# The caller may supply an isolated scratch target/cache for sandboxed execution.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/.build/glib-regression-target}"
export CARGO_NET_OFFLINE=true
exec cargo test --frozen --release --jobs 2 \
  --manifest-path "$ROOT/tests/glib-regression/Cargo.toml" \
  --test variant_str_iter -- --test-threads=1 --nocapture
