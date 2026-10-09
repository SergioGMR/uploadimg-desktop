#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/.build/cargo-target"
export CARGO_NET_OFFLINE=true
export DYLD_LIBRARY_PATH="$ROOT/.build/native"
cargo test --offline --locked --jobs 2 --manifest-path "$ROOT/Cargo.toml" --workspace
bash "$ROOT/scripts/test-native.sh"
node --test "$ROOT"/ui/tests/*.test.mjs
python3 "$ROOT/scripts/tests/test-package-manifest.py"
python3 "$ROOT/scripts/tests/test-updater-package.py"
