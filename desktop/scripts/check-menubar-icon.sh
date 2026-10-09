#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$ROOT/.build/ModuleCache" "$ROOT/.build/menubar-icon"
xcrun swiftc -swift-version 5 -module-cache-path "$ROOT/.build/ModuleCache" \
  "$ROOT/native/macos/Images.swift" "$ROOT/native/macos/SyntheticImage.swift" \
  "$ROOT/native/macos/Bridge.swift" "$ROOT/native/Tests/MenuBarIconPreview/main.swift" \
  -o "$ROOT/.build/menubar-icon-check"
"$ROOT/.build/menubar-icon-check" "$ROOT/.build/menubar-icon"
