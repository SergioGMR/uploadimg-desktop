#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/.build/cargo-target"
export CARGO_NET_OFFLINE=true
python3 "$ROOT/scripts/generate-icon.py"
cargo build --offline --locked --jobs 2 --manifest-path "$ROOT/Cargo.toml" -p uploadimg-desktop
APP="$ROOT/.build/UploadIMG.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Frameworks" "$APP/Contents/Resources"
cp "$CARGO_TARGET_DIR/debug/uploadimg-desktop" "$APP/Contents/MacOS/UploadimgDesktop"
cp "$ROOT/.build/native/libUploadimgNative.dylib" "$APP/Contents/Frameworks/libUploadimgNative.dylib"
cp "$ROOT/src-tauri/Info.plist" "$APP/Contents/Info.plist"
cp "$ROOT/src-tauri/icons/icon.icns" "$APP/Contents/Resources/icon.icns"
plutil -lint "$APP/Contents/Info.plist"
codesign --force --sign - --identifier local.uploadimg.desktop.m1.native "$APP/Contents/Frameworks/libUploadimgNative.dylib"
codesign --force --sign - --identifier local.uploadimg.desktop.m1 "$APP"
codesign --verify --deep --strict "$APP"
otool -L "$APP/Contents/MacOS/UploadimgDesktop"
otool -L "$APP/Contents/Frameworks/libUploadimgNative.dylib"
otool -l "$APP/Contents/MacOS/UploadimgDesktop" | awk '/LC_RPATH/{show=3} show{print;show--}'
echo "PASS: offline app bundle built, native component signed first, deep/strict signature verified. No launch."
