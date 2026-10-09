"""Check the editable icon resources and local macOS bundle wiring."""
from pathlib import Path
import json
import plistlib
import struct

root = Path(__file__).resolve().parents[3]
plist = plistlib.loads((root / 'src-tauri/Info.plist').read_bytes())
assert plist['CFBundleName'] == 'UploadIMG'
assert plist['CFBundleDisplayName'] == 'UploadIMG'
assert plist['CFBundleIconFile'] == 'icon.icns'
config = json.loads((root / 'src-tauri/tauri.conf.json').read_text())
assert config['productName'] == 'UploadIMG'
assert config['app']['windows'][0]['title'] == 'UploadIMG'
for filename, size in [('icon.png', 1024), ('32x32.png', 32), ('128x128.png', 128), ('128x128@2x.png', 256)]:
    png = (root / 'src-tauri/icons' / filename).read_bytes()
    assert png[:8] == b'\x89PNG\r\n\x1a\n', filename
    assert struct.unpack('>II', png[16:24]) == (size, size), filename
    assert png[25] == 6, f'{filename} must retain RGBA transparency'
icns = (root / 'src-tauri/icons/icon.icns').read_bytes()
assert icns[:4] == b'icns' and struct.unpack('>I', icns[4:8])[0] == len(icns)
representations = set()
offset = 8
while offset < len(icns):
    kind, length = struct.unpack('>4sI', icns[offset:offset + 8])
    assert length >= 8 and offset + length <= len(icns)
    representations.add(kind)
    offset += length
assert offset == len(icns)
assert {b'ic07', b'ic08', b'ic09', b'ic10'}.issubset(representations)
build = (root / 'scripts/build.sh').read_text()
assert 'APP="$ROOT/.build/UploadIMG.app"' in build
assert 'cp "$ROOT/src-tauri/icons/icon.icns" "$APP/Contents/Resources/icon.icns"' in build
bridge = (root / 'native/macos/Bridge.swift').read_text()
assert 'toolTip = "UploadIMG"' in bridge
assert 'setAccessibilityLabel("UploadIMG")' in bridge
assert '"Abrir UploadIMG"' in bridge
print('PASS: UploadIMG metadata, RGBA icon sizes, ICNS representations and bundle resource wiring')
