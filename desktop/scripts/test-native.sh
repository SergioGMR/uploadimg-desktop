#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$ROOT/.build/ModuleCache"
xcrun swiftc -swift-version 5 -module-cache-path "$ROOT/.build/ModuleCache" \
  "$ROOT/native/macos/Images.swift" "$ROOT/native/macos/SyntheticImage.swift" \
  "$ROOT/native/macos/Bridge.swift" "$ROOT/native/macos/Upload.swift" "$ROOT/native/Tests/main.swift" -o "$ROOT/.build/native-tests"
"$ROOT/.build/native-tests"
