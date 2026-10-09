#!/bin/bash
# Package the reviewed local debug bundle; never build or re-sign it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/scripts/package-manifest.py"
VERSION="$(python3 "$MANIFEST" version)"
NAME="UploadIMG-${VERSION}-macos-arm64.dmg"
APP="$ROOT/.build/UploadIMG.app"
BASE="$ROOT/.build/distribution"
OUTPUT="${1:-$BASE/$VERSION}"
for tool in hdiutil ditto codesign plutil otool lipo shasum python3; do
    command -v "$tool" >/dev/null || { echo "Missing local tool: $tool" >&2; exit 1; }
done
python3 "$MANIFEST" guard "$BASE" "$OUTPUT" "$NAME"
mkdir -p "$OUTPUT"
OUTPUT="$(cd "$OUTPUT" && pwd)"
# Claim the log exclusively as the reservation for this output directory.
(set -o noclobber; : > "$OUTPUT/packaging-verification.txt")
WORK="$(mktemp -d "$OUTPUT/.packaging.XXXXXX")"
MOUNT_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/uploadimg-dmg-mount.XXXXXX")"
MOUNT_ROOT="$(cd "$MOUNT_ROOT" && pwd -P)"
MOUNT="$MOUNT_ROOT/volume"
MOUNTED=0
DEVICE=""
cleanup() {
    status=$?
    if [[ "$MOUNTED" == 1 ]]; then
        if [[ -z "$DEVICE" ]]; then
            DEVICE="$(python3 "$MANIFEST" attachment "$WORK/attach.plist" "$MOUNT")" || DEVICE=""
        fi
        if [[ -z "$DEVICE" ]] || ! hdiutil detach "$DEVICE"; then
            echo "Detach failed; retain exact attachment receipt at $WORK/attach.plist" >&2
            status=1
        fi
    fi
    if [[ "$MOUNTED" == 0 ]]; then
        rmdir "$MOUNT" "$MOUNT_ROOT" 2>/dev/null || true
    fi
    # Keep our staging and receipts for inspection; never remove other outputs.
    exit "$status"
}
trap cleanup EXIT
exec > >(tee -a "$OUTPUT/packaging-verification.txt") 2>&1
echo "Distribution label: $VERSION; packaging $APP."
echo "Existing local debug bundle; no build, launch, install, or re-sign."
plutil -lint "$APP/Contents/Info.plist"
codesign --verify --deep --strict "$APP"
codesign -dv --verbose=4 "$APP"
python3 "$MANIFEST" capture "$APP" "$WORK/source-manifest.json"
mkdir -p "$WORK/stage" "$MOUNT"
ditto --noextattr --norsrc "$APP" "$WORK/stage/UploadIMG.app"
ln -s /Applications "$WORK/stage/Applications"
ditto --noextattr --norsrc "$ROOT/distribution/README.md" "$WORK/stage/README.txt"
ditto --noextattr --norsrc "$ROOT/distribution/release-notes.md" "$WORK/stage/RELEASE-NOTES.txt"
codesign --verify --deep --strict "$WORK/stage/UploadIMG.app"
hdiutil create -srcfolder "$WORK/stage" -volname "UploadIMG $VERSION" \
    -fs HFS+ -format UDZO "$WORK/$NAME"
hdiutil verify "$WORK/$NAME"
# Mark the attempt before attaching so partial failures retain/inspect the receipt.
MOUNTED=1
hdiutil attach -readonly -nobrowse -mountpoint "$MOUNT" -plist "$WORK/$NAME" > "$WORK/attach.plist"
DEVICE="$(python3 "$MANIFEST" attachment "$WORK/attach.plist" "$MOUNT")"
echo "Read-only/nobrowse attachment: $DEVICE at $MOUNT"
codesign --verify --deep --strict "$MOUNT/UploadIMG.app"
python3 "$MANIFEST" compare "$APP" "$MOUNT" "$WORK/source-manifest.json"
hdiutil detach "$DEVICE"
MOUNTED=0
# Hard links fail when a destination exists, unlike an overwriting mv/cp.
ln "$WORK/$NAME" "$OUTPUT/$NAME"
python3 "$MANIFEST" provenance "$OUTPUT/$NAME" "$WORK/source-manifest.json" "$OUTPUT/provenance.json"
(cd "$OUTPUT" && (set -o noclobber; shasum -a 256 "$NAME" provenance.json > SHA256SUMS) && shasum -a 256 -c SHA256SUMS)
stat -f 'DMG bytes: %z' "$OUTPUT/$NAME"
echo "PASS: UDZO DMG verified and exact device detached; checksums validated."
echo "NOT_RUN: installed-app launch, another Mac, download quarantine/Gatekeeper, notarization."
