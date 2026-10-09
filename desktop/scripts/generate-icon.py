"""Generate the editable macOS app icon using the installed Apple toolchain."""
from pathlib import Path
import shutil
import subprocess

root = Path(__file__).resolve().parent.parent
icons = root / 'src-tauri/icons'
work = root / '.build/app-icon'
iconset = work / 'icon.iconset'
cache = work / 'ModuleCache'
icons.mkdir(parents=True, exist_ok=True)
cache.mkdir(parents=True, exist_ok=True)
subprocess.run([
    '/usr/bin/xcrun', 'swift', '-module-cache-path', str(cache),
    str(root / 'scripts/generate-app-icon.swift'), str(iconset),
], check=True)
for source, destination in [
    ('icon_512x512@2x.png', 'icon.png'),
    ('icon_32x32.png', '32x32.png'),
    ('icon_128x128.png', '128x128.png'),
    ('icon_128x128@2x.png', '128x128@2x.png'),
]:
    shutil.copyfile(iconset / source, icons / destination)
subprocess.run([
    '/usr/bin/iconutil', '--convert', 'icns', str(iconset),
    '--output', str(icons / 'icon.icns'),
], check=True)
print('PASS: generated UploadIMG PNG resources and complete macOS ICNS')
