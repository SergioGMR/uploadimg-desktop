"""Local packaging receipts and guards; never infer a bundle's compilation origin."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import plistlib
import re
import stat
import subprocess
import sys

REPO = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('check_version', REPO / 'scripts/check-version.py')
versions = importlib.util.module_from_spec(spec)
spec.loader.exec_module(versions)

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def write_json(path, data):
    with path.open('x') as target:
        json.dump(data, target, sort_keys=True, indent=2)
        target.write('\n')

def guard_output(base, output, name):
    base, output = base.resolve(), output.resolve()
    if output != base and base not in output.parents:
        raise ValueError('Output must remain inside desktop/.build/distribution')
    for filename in [name, 'SHA256SUMS', 'provenance.json', 'packaging-verification.txt']:
        target = output / filename
        if target.exists() or target.is_symlink():
            raise ValueError(f'Refusing to overwrite existing output: {target}')

def validate_info(info, expected, version):
    fields = ['CFBundleIdentifier', 'CFBundleShortVersionString', 'CFBundleVersion',
              'CFBundleExecutable', 'CFBundleName', 'CFBundleDisplayName',
              'CFBundleIconFile', 'LSMinimumSystemVersion']
    if any(info.get(key) != expected.get(key) for key in fields):
        raise ValueError('Bundle metadata differs from current source plist; rebuild and review it')
    required = {'CFBundleIdentifier': 'local.uploadimg.desktop.m1',
                'CFBundleName': 'UploadIMG', 'CFBundleDisplayName': 'UploadIMG',
                'CFBundleExecutable': 'UploadimgDesktop', 'CFBundleIconFile': 'icon.icns',
                'LSMinimumSystemVersion': '15.4'}
    if any(info.get(key) != value for key, value in required.items()):
        raise ValueError('Unexpected UploadIMG bundle identity or platform floor')
    if versions.parse_version(version) != info['CFBundleShortVersionString']:
        raise ValueError('VERSION base differs from bundle CFBundleShortVersionString')

def bundle_entries(app):
    if not app.is_dir() or app.is_symlink():
        raise ValueError(f'Missing regular application bundle: {app}')
    entries = {}
    for path in sorted(app.rglob('*')):
        relative = path.relative_to(app).as_posix()
        mode = stat.S_IMODE(path.lstat().st_mode)
        if path.is_symlink():
            target = os.readlink(path)
            if os.path.isabs(target) or not path.resolve().is_relative_to(app.resolve()) or not path.exists():
                raise ValueError(f'Unsafe or dangling bundle symlink: {relative}')
            entries[relative] = {'symlink': target}
        elif path.is_file(): entries[relative] = {'sha256': digest(path), 'mode': mode}
        elif path.is_dir(): entries[relative] = {'directory': True, 'mode': mode}
        else: raise ValueError(f'Unsupported bundle entry: {relative}')
    return entries

def source_snapshot(root):
    def git(*args):
        return subprocess.check_output(['git', '-C', str(root), *args])
    paths = sorted(set(git('ls-files', '--cached', '--others', '--exclude-standard', '-z').split(b'\0')) - {b''})
    entries = []
    for raw in paths:
        relative = os.fsdecode(raw)
        path = root / relative
        if path.is_symlink(): value = {'symlink': os.readlink(path)}
        elif path.is_file(): value = {'sha256': digest(path)}
        elif not path.exists(): value = {'deleted': True}
        else: raise ValueError(f'Unsupported source entry: {relative}')
        entries.append({'path': relative, **value})
    encoded = json.dumps(entries, sort_keys=True, separators=(',', ':')).encode()
    status = git('status', '--porcelain=v1', '-z').decode(errors='replace').split('\0')
    return {'git_head': git('rev-parse', 'HEAD').decode().strip(),
            'git_dirty': any(status), 'git_status': [s for s in status if s],
            'algorithm': 'SHA256 of UTF-8 sorted compact JSON source file records',
            'sha256': hashlib.sha256(encoded).hexdigest(), 'files': entries}

def attachment(receipt, mount):
    entities = receipt['system-entities']
    devices = [e['dev-entry'] for e in entities if re.fullmatch(r'/dev/disk[0-9]+', e.get('dev-entry', ''))]
    mounts = [e['mount-point'] for e in entities if 'mount-point' in e]
    if len(devices) != 1 or mounts != [str(mount)]:
        raise ValueError('Unexpected attached device or mountpoint; retain attach.plist for inspection')
    return devices[0]

def inspect_binary(binary, executable):
    if subprocess.check_output(['lipo', '-archs', str(binary)], text=True).strip() != 'arm64':
        raise ValueError(f'Unexpected architecture: {binary}')
    load = subprocess.check_output(['otool', '-L', str(binary)], text=True)
    for line in load.splitlines()[1:]:
        dependency = line.strip().split(' (', 1)[0]
        if not dependency.startswith(('/System/Library/', '/usr/lib/')) and dependency != '@rpath/libUploadimgNative.dylib':
            raise ValueError(f'Nonportable dependency: {dependency}')
    commands = subprocess.check_output(['otool', '-l', str(binary)], text=True).splitlines()
    rpaths = [commands[i + 2].strip().split(' (', 1)[0].removeprefix('path ')
              for i, line in enumerate(commands) if line.strip() == 'cmd LC_RPATH']
    if rpaths != (['@executable_path/../Frameworks'] if executable else []):
        raise ValueError(f'Unexpected rpaths: {rpaths}')
    signature = subprocess.run(['codesign', '-dv', '--verbose=4', str(binary)], capture_output=True, text=True, check=True)
    if 'Signature=adhoc' not in signature.stderr:
        raise ValueError(f'Unexpected signing identity: {binary}')

def main():
    command, *args = sys.argv[1:]
    root = REPO / 'desktop'
    if command == 'version':
        print(versions.check(REPO))
    elif command == 'guard':
        guard_output(Path(args[0]), Path(args[1]), args[2])
    elif command == 'attachment':
        print(attachment(plistlib.loads(Path(args[0]).read_bytes()), Path(args[1])))
    elif command == 'capture':
        app, manifest = map(Path, args[:2])
        version = versions.check(REPO)
        info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
        validate_info(info, plistlib.loads((root / 'src-tauri/Info.plist').read_bytes()), version)
        if digest(app / 'Contents/Resources/icon.icns') != digest(root / 'src-tauri/icons/icon.icns'):
            raise ValueError('Bundle icon differs from current source icon')
        inspect_binary(app / 'Contents/MacOS/UploadimgDesktop', True)
        inspect_binary(app / 'Contents/Frameworks/libUploadimgNative.dylib', False)
        write_json(manifest, {'version': version, 'source_at_packaging': source_snapshot(REPO),
                             'bundle_info': info, 'bundle': bundle_entries(app)})
        print('PASS: current bundle metadata/icon, arm64, portable dependencies/rpaths and ad-hoc binary signatures')
    elif command == 'compare':
        app, mount, manifest = map(Path, args)
        baseline = json.loads(manifest.read_text())
        if source_snapshot(REPO) != baseline['source_at_packaging']:
            raise ValueError('Source changed during packaging')
        if bundle_entries(app) != baseline['bundle'] or bundle_entries(mount / 'UploadIMG.app') != baseline['bundle']:
            raise ValueError('Source bundle changed or mounted bundle differs')
        link = mount / 'Applications'
        if not link.is_symlink() or os.readlink(link) != '/Applications':
            raise ValueError('Missing /Applications installation symlink')
        for source, target in [('README.md', 'README.txt'), ('release-notes.md', 'RELEASE-NOTES.txt')]:
            if digest(root / 'distribution' / source) != digest(mount / target):
                raise ValueError(f'Mounted instruction mismatch: {target}')
        print('PASS: unchanged source/bundle, mounted bundle hashes, Applications symlink and installation notes')
    elif command == 'provenance':
        dmg, manifest, output = map(Path, args)
        baseline = json.loads(manifest.read_text())
        write_json(output, {**baseline, 'compiled_source_commit': None,
            'build_origin': 'Not inferred: git HEAD/source digest describe packaging-time sources only',
            'profile': 'debug', 'signing': 'ad-hoc', 'notarized': False,
            'artifact': {'path': dmg.name, 'sha256': digest(dmg)},
            'verified': ['bundle metadata/icon', 'arm64/signature/dependency guards',
                         'UDZO integrity', 'read-only mounted byte comparison', 'exact device detached'],
            'not_run': ['installed-app launch', 'another Mac', 'download quarantine/Gatekeeper']})
    else:
        raise ValueError(f'Unknown command: {command}')

if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'Packaging check failed: {error}')
