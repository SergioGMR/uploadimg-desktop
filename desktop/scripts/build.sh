#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export CARGO_TARGET_DIR="$ROOT/.build/cargo-target"
export CARGO_NET_OFFLINE=true
python3 "$ROOT/scripts/generate-icon.py"
RECEIPT_WORK="$(mktemp -d "$ROOT/.build/build-receipt.XXXXXX")"
# Invalidate the old receipt before any compilation attempt; failure must not reuse it.
rm -f "$ROOT/.build/build-source.json"
python3 "$ROOT/scripts/package-updater.py" build-start "$RECEIPT_WORK/start.json"
cargo build --offline --locked --jobs 2 --manifest-path "$ROOT/Cargo.toml" -p uploadimg-desktop
APP="$ROOT/.build/UploadIMG.app"
# Recreate only this known build output so stale resources cannot enter a new bundle.
if [[ -L "$APP" ]]; then echo "Refusing a symlink at app build output" >&2; exit 1; fi
rm -rf "$APP"
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
python3 "$ROOT/scripts/package-updater.py" build-finish "$RECEIPT_WORK/start.json" "$APP" "$RECEIPT_WORK/verified.json"
mv "$RECEIPT_WORK/verified.json" "$ROOT/.build/build-source.json"
echo "PASS: offline app bundle built, native component signed first, deep/strict signature verified. No launch."
