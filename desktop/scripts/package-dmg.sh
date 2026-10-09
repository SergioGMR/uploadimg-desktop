#!/bin/bash
# Package the already-tested debug bundle; never build or re-sign it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="$(python3 - "$ROOT/../VERSION" <<'PY'
from pathlib import Path
import re, sys
version = Path(sys.argv[1]).read_text().strip()
number = r'(?:0|[1-9][0-9]*)'
identifier = r'(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)'
pattern = rf'{number}\.{number}\.{number}(?:-{identifier}(?:\.{identifier})*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?'
if not re.fullmatch(pattern, version):
    raise SystemExit('VERSION must contain one strict SemVer value')
print(version)
PY
)"
NAME="Uploadimg-Desktop-${VERSION}-macos-arm64.dmg"
APP="$ROOT/.build/Uploadimg Desktop.app"
BASE="$ROOT/.build/distribution"
OUTPUT="${1:-$BASE}"
for tool in hdiutil ditto codesign plutil otool lipo shasum python3; do
    command -v "$tool" >/dev/null || { echo "Missing local tool: $tool" >&2; exit 1; }
done
python3 - "$BASE" "$OUTPUT" <<'PY'
from pathlib import Path
import sys
base, output = (Path(p).resolve() for p in sys.argv[1:])
if output != base and base not in output.parents:
    raise SystemExit('Output must remain inside desktop/.build/distribution')
PY
mkdir -p "$OUTPUT"
OUTPUT="$(cd "$OUTPUT" && pwd)"
for path in "$OUTPUT/$NAME" "$OUTPUT/SHA256SUMS" "$OUTPUT/provenance.json" "$OUTPUT/packaging-verification.txt"; do
    if [[ -e "$path" || -L "$path" ]]; then
        echo "Refusing to overwrite existing output: $path. Use a new output subdirectory." >&2
        exit 1
    fi
done
WORK="$(mktemp -d "$OUTPUT/.packaging.XXXXXX")"
MOUNT="$WORK/mount"
MOUNTED=0
DEVICE=""
cleanup() {
    status=$?
    if [[ "$MOUNTED" == 1 ]]; then
        if [[ -z "$DEVICE" ]]; then
            DEVICE="$(python3 - "$WORK/attach.plist" <<'PY'
import plistlib, re, sys
with open(sys.argv[1], 'rb') as file: entities = plistlib.load(file)['system-entities']
devices = [e['dev-entry'] for e in entities if re.fullmatch(r'/dev/disk[0-9]+', e.get('dev-entry', ''))]
if len(devices) != 1: raise SystemExit('Cannot identify exact attached device')
print(devices[0])
PY
)"
        fi
        if [[ -z "$DEVICE" ]] || ! hdiutil detach "$DEVICE"; then
            echo "Detach failed; inspect retained attach.plist at $WORK" >&2
            status=1
        fi
    fi
    # Retain only our staging/evidence; never delete existing files or mounts.
    exit "$status"
}
trap cleanup EXIT
exec > >(tee "$OUTPUT/packaging-verification.txt") 2>&1
echo "Distribution label: $VERSION; app version remains 0.1.0 (build 1)."
echo "Packaging existing debug bundle; no build, launch, install, or re-sign."
plutil -lint "$APP/Contents/Info.plist"
codesign --verify --deep --strict "$APP"
codesign -dv --verbose=4 "$APP"
python3 - "$ROOT" "$APP" "$WORK/source-manifest.json" "$VERSION" <<'PY'
from pathlib import Path
import hashlib, json, plistlib, subprocess, sys
root, app, manifest = map(Path, sys.argv[1:4])
version = sys.argv[4]
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
snapshot = root / 'evidence/source-snapshot.json'
if digest(snapshot) != '7888699c8b2581275502aa5bf3edcff7ac20d220a05686cf248cb75c60206419':
    raise SystemExit('Source snapshot has changed; this script packages only the tested beta boundary')
info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
expected = {'CFBundleIdentifier': 'local.uploadimg.desktop.m1',
            'CFBundleShortVersionString': '0.1.0', 'CFBundleVersion': '1',
            'CFBundleExecutable': 'UploadimgDesktop', 'LSMinimumSystemVersion': '15.4'}
if any(info.get(k) != v for k, v in expected.items()):
    raise SystemExit('Unexpected bundle identity/version/platform floor')
if version.split('-', 1)[0].split('+', 1)[0] != info['CFBundleShortVersionString']:
    raise SystemExit('VERSION base differs from bundle CFBundleShortVersionString')
snapshot_data = json.loads(snapshot.read_text())
expected_files = {}
for item in snapshot_data['artifacts']:
    p = root / item['path']
    if digest(p) != item['sha256']:
        raise SystemExit(f'Tested artifact hash mismatch: {p}')
    expected_files[str(p.relative_to(app))] = item['sha256']
if digest(app / 'Contents/MacOS/UploadimgDesktop') != '2641d50045d82b2a13a51286d77c8b322903e1cc17be8f4cc09ea8f0f19db386':
    raise SystemExit('Unexpected executable hash')
entries = {}
for p in sorted(app.rglob('*')):
    relative = str(p.relative_to(app))
    if p.is_symlink():
        raise SystemExit(f'Unexpected source bundle symlink: {relative}')
    if p.is_file(): entries[relative] = {'sha256': digest(p)}
    elif p.is_dir(): entries[relative] = {'directory': True}
    else: raise SystemExit(f'Unsupported bundle entry: {relative}')
if {p: v['sha256'] for p, v in entries.items() if 'sha256' in v} != expected_files:
    raise SystemExit('Bundle file set differs from tested source-snapshot artifacts')
for relative in ['Contents/MacOS/UploadimgDesktop', 'Contents/Frameworks/libUploadimgNative.dylib']:
    binary = app / relative
    if subprocess.check_output(['lipo', '-archs', str(binary)], text=True).strip() != 'arm64':
        raise SystemExit(f'Unexpected architecture: {relative}')
    load = subprocess.check_output(['otool', '-L', str(binary)], text=True)
    print(load, end='')
    for line in load.splitlines()[1:]:
        dependency = line.strip().split(' (', 1)[0]
        if not dependency.startswith(('/System/Library/', '/usr/lib/')) and dependency != '@rpath/libUploadimgNative.dylib':
            raise SystemExit(f'Nonportable dependency: {dependency}')
    commands = subprocess.check_output(['otool', '-l', str(binary)], text=True).splitlines()
    rpaths = [commands[i + 2].strip().split(' (', 1)[0].removeprefix('path ')
              for i, line in enumerate(commands) if line.strip() == 'cmd LC_RPATH']
    if relative.startswith('Contents/MacOS/') and rpaths != ['@executable_path/../Frameworks']:
        raise SystemExit(f'Unexpected executable rpaths: {rpaths}')
    if relative.startswith('Contents/Frameworks/') and rpaths:
        raise SystemExit(f'Unexpected native rpaths: {rpaths}')
    signature = subprocess.run(['codesign', '-dv', '--verbose=4', str(binary)], capture_output=True, text=True, check=True)
    if 'Signature=adhoc' not in signature.stderr:
        raise SystemExit(f'Unexpected signing identity: {relative}')
manifest.write_text(json.dumps(entries, sort_keys=True, indent=2) + '\n')
print('PASS: exact tested file set/hashes, identity, arm64, portable dylibs/rpaths, ad-hoc signatures.')
print('Source snapshot identity:', snapshot_data['snapshot'], '(not a Git commit)')
PY
mkdir -p "$WORK/stage"
ditto --noextattr --norsrc "$APP" "$WORK/stage/Uploadimg Desktop.app"
# Only the distribution copy gets standard readable/executable permissions.
find "$WORK/stage/Uploadimg Desktop.app" -type d -exec chmod 755 {} +
find "$WORK/stage/Uploadimg Desktop.app" -type f -exec chmod 644 {} +
chmod 755 "$WORK/stage/Uploadimg Desktop.app/Contents/MacOS/UploadimgDesktop" \
    "$WORK/stage/Uploadimg Desktop.app/Contents/Frameworks/libUploadimgNative.dylib"
ln -s /Applications "$WORK/stage/Applications"
ditto --noextattr --norsrc "$ROOT/distribution/README.md" "$WORK/stage/README.txt"
ditto --noextattr --norsrc "$ROOT/distribution/release-notes.md" "$WORK/stage/RELEASE-NOTES.txt"
chmod 755 "$WORK/stage"
chmod 644 "$WORK/stage/README.txt" "$WORK/stage/RELEASE-NOTES.txt"
codesign --verify --deep --strict "$WORK/stage/Uploadimg Desktop.app"
hdiutil create -srcfolder "$WORK/stage" -volname "Uploadimg Desktop $VERSION" \
    -fs HFS+ -format UDZO "$WORK/$NAME"
hdiutil verify "$WORK/$NAME"
hdiutil attach -readonly -nobrowse -plist "$WORK/$NAME" > "$WORK/attach.plist"
MOUNTED=1
IFS=$'\t' read -r DEVICE MOUNT < <(python3 - "$WORK/attach.plist" <<'PY'
from pathlib import Path
import plistlib, re, sys
with open(sys.argv[1], 'rb') as file: entities = plistlib.load(file)['system-entities']
devices = [e['dev-entry'] for e in entities if re.fullmatch(r'/dev/disk[0-9]+', e.get('dev-entry', ''))]
mounts = [e['mount-point'] for e in entities if 'mount-point' in e]
if len(devices) != 1 or len(mounts) != 1 or Path(mounts[0]).parent != Path('/Volumes'):
    raise SystemExit('Unexpected attached device or mountpoint; retaining attachment receipt')
print(devices[0] + '\t' + mounts[0])
PY
)
echo "Read-only/nobrowse attachment: $DEVICE at $MOUNT"
codesign --verify --deep --strict "$MOUNT/Uploadimg Desktop.app"
python3 - "$APP" "$MOUNT" "$WORK/source-manifest.json" "$ROOT/distribution" <<'PY'
from pathlib import Path
import hashlib, json, os, sys
source, mount, manifest, docs = map(Path, sys.argv[1:])
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
def entries(app):
    result = {}
    for p in sorted(app.rglob('*')):
        relative = str(p.relative_to(app))
        if p.is_symlink(): raise SystemExit(f'Unexpected bundle symlink: {relative}')
        result[relative] = {'sha256': digest(p)} if p.is_file() else {'directory': True}
    return result
baseline = json.loads(manifest.read_text())
if entries(source) != baseline or entries(mount / 'Uploadimg Desktop.app') != baseline:
    raise SystemExit('Source changed or mounted app differs from exact tested bytes/file set')
if not (mount / 'Applications').is_symlink() or os.readlink(mount / 'Applications') != '/Applications':
    raise SystemExit('Missing /Applications installation symlink')
for source_name, disk_name in [('README.md', 'README.txt'), ('release-notes.md', 'RELEASE-NOTES.txt')]:
    if digest(docs / source_name) != digest(mount / disk_name):
        raise SystemExit(f'Mounted instruction mismatch: {disk_name}')
print('PASS: read-only mounted app and unchanged source match every baseline file/hash/directory; installation symlink and docs match.')
PY
hdiutil detach "$DEVICE"
MOUNTED=0
mv "$WORK/$NAME" "$OUTPUT/$NAME"
python3 - "$ROOT" "$OUTPUT/$NAME" "$WORK/source-manifest.json" <<'PY'
from pathlib import Path
import hashlib, json, sys
root, dmg, manifest = map(Path, sys.argv[1:])
snapshot = root / 'evidence/source-snapshot.json'
hash_file = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
data = {
    'packaged_source_commit': None,
    'built_from_reviewed_snapshot': {'path': 'desktop/evidence/source-snapshot.json',
                        'sha256': hash_file(snapshot),
                        'identity': json.loads(snapshot.read_text())['snapshot']},
    'bundle': [{'path': 'Uploadimg Desktop.app/' + path, 'sha256': value['sha256']}
               for path, value in json.loads(manifest.read_text()).items() if 'sha256' in value],
    'artifacts': [{'path': dmg.name, 'sha256': hash_file(dmg)}]
}
(dmg.parent / 'provenance.json').write_text(json.dumps(data, sort_keys=True, indent=2) + '\n')
PY
(cd "$OUTPUT" && shasum -a 256 "$NAME" provenance.json > SHA256SUMS && shasum -a 256 -c SHA256SUMS)
stat -f 'DMG bytes: %z' "$OUTPUT/$NAME"
echo "PASS: UDZO DMG verified and cleanly detached; checksum validated."
echo "NOT_RUN: installed-app launch, another Mac, download quarantine/Gatekeeper, notarization."
