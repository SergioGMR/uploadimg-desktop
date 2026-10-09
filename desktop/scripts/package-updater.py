#!/usr/bin/env python3
"""Package the existing custom macOS bundle; official Tauri CLI performs signing."""
import argparse
import base64
import binascii
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import plistlib
import re
import stat
import subprocess
import tarfile
import tempfile

spec = importlib.util.spec_from_file_location('package_manifest', Path(__file__).with_name('package-manifest.py'))
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)
REPO = packaging.REPO


def guard_output(base, output):
    base, output = base.resolve(), output.resolve()
    if base not in output.parents or output.exists() or output.is_symlink():
        raise ValueError('Updater output must be a new directory inside desktop/.build/distribution')


def validate_build_receipt(receipt, root, app, version):
    if (receipt.get('schema') != 1 or receipt.get('version') != version
            or receipt.get('source') != packaging.source_snapshot(root)
            or receipt.get('bundle') != packaging.bundle_entries(app)):
        raise ValueError('Missing/stale build-source receipt: rebuild and review the exact source and bundle')


def create_archive(app, archive):
    packaging.bundle_entries(app)  # Reject unsafe links before archive creation.
    with archive.open('xb') as output, tarfile.open(fileobj=output, mode='w:gz') as target:
        target.add(app, arcname='UploadIMG.app', recursive=False)
        for path in sorted(app.rglob('*')):
            target.add(path, arcname='UploadIMG.app/' + path.relative_to(app).as_posix(), recursive=False)
    if archive.stat().st_size > 256 * 1024 * 1024:
        raise ValueError('Updater archive exceeds the application 256 MiB download limit')


def verify_archive(archive, app, destination):
    expected = {'UploadIMG.app', *('UploadIMG.app/' + name for name in packaging.bundle_entries(app))}
    with tarfile.open(archive, 'r:gz') as source:
        members = source.getmembers()
        names = [member.name for member in members]
        if len(names) != len(set(names)) or set(names) != expected:
            raise ValueError('Updater archive must contain exactly the reviewed UploadIMG.app layout')
        for member in members:
            path = PurePosixPath(member.name)
            if path.is_absolute() or '..' in path.parts or not (member.isdir() or member.isfile() or member.issym()):
                raise ValueError('Unsafe updater archive entry')
            original = app.parent / member.name
            if (member.issym() != original.is_symlink()
                    or member.isdir() != (original.is_dir() and not original.is_symlink())
                    or member.isfile() != (original.is_file() and not original.is_symlink())):
                raise ValueError('Updater archive entry type differs from reviewed bundle')
            if member.issym():
                if member.linkname != os.readlink(original):
                    raise ValueError('Updater archive symlink differs from reviewed bundle')
            elif member.mode != stat.S_IMODE(original.stat().st_mode):
                raise ValueError('Updater archive permissions differ from reviewed bundle')
            if member.isfile() and member.size != original.stat().st_size:
                raise ValueError('Updater archive member size differs from reviewed bundle')
        destination.mkdir()
        source.extractall(destination, filter='data')
        # Python's data filter deliberately normalizes directory/writable bits.
        # Restore only modes already proven equal to the safe reviewed source;
        # Tauri's tar extractor receives those original modes in the archive.
        for member in members:
            if not member.issym():
                (destination / member.name).chmod(member.mode)
    if packaging.bundle_entries(destination / 'UploadIMG.app') != packaging.bundle_entries(app):
        raise ValueError('Extracted updater archive changed bytes, permissions or symlinks')


def decode_lines(value):
    if not isinstance(value, str) or not value or len(value) > 8192:
        raise ValueError('Missing or oversized updater signature/public key contents')
    try:
        return base64.b64decode(value.strip(), validate=True).decode('utf-8').splitlines()
    except (binascii.Error, UnicodeError) as error:
        raise ValueError('Invalid updater signature/public key encoding') from error


def make_manifest(version, signature, public_key):
    """Validate structure/key ID/version only; this is NOT cryptographic verification."""
    packaging.versions.parse_version(version)
    try:
        public = decode_lines(public_key)
        signed = decode_lines(signature)
        if (len(public) != 2 or len(signed) != 4 or not public[0].startswith('untrusted comment: ')
                or not signed[0].startswith('untrusted comment: ') or not signed[2].startswith('trusted comment: ')):
            raise ValueError('Invalid minisign text structure')
        public_bytes = base64.b64decode(public[1], validate=True)
        signed_bytes = base64.b64decode(signed[1], validate=True)
        global_signature = base64.b64decode(signed[3], validate=True)
        if (len(public_bytes) != 42 or public_bytes[:2] != b'Ed' or len(signed_bytes) != 74
                or signed_bytes[:2] not in (b'Ed', b'ED') or len(global_signature) != 64
                or public_bytes[2:10] != signed_bytes[2:10]):
            raise ValueError('Updater signature/public key structure or key ID mismatch')
        fields = signed[2].removeprefix('trusted comment: ').split('\t')
        if [field for field in fields if field.startswith('version:')] != ['version:' + version]:
            raise ValueError('Signature must bind the exact distribution version')
    except (IndexError, binascii.Error) as error:
        raise ValueError('Invalid minisign signature/public key') from error
    name = f'UploadIMG-{version}-macos-arm64.app.tar.gz'
    manifest = {'version': version, 'platforms': {'darwin-aarch64': {
        'url': f'https://github.com/SergioGMR/uploadimg-desktop/releases/download/v{version}/{name}',
        'signature': signature.strip()}}}
    if len(json.dumps(manifest).encode()) > 65536:
        raise ValueError('Generated updater feed exceeds the 64 KiB publication limit')
    return manifest


def sign_archive(archive, signer, key, version):
    # Never read key contents in Python or pass contents/password through argv/logs.
    if not key.is_file() or key.is_symlink() or stat.S_IMODE(key.stat().st_mode) & 0o077:
        raise ValueError('A regular private key file with owner-only permissions is required')
    if key.resolve().is_relative_to(REPO):
        raise ValueError('The updater private key must remain outside the repository')
    if Path(str(archive) + '.sig').exists():
        raise ValueError('Refusing to overwrite an updater signature')
    env = os.environ.copy()
    for name in ('TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PATH',
                 'TAURI_PRIVATE_KEY', 'TAURI_PRIVATE_KEY_PATH', 'TAURI_PRIVATE_KEY_PASSWORD'):
        env.pop(name, None)
    tool = subprocess.run([str(signer), '--version'], env=env, stdin=subprocess.DEVNULL,
                          capture_output=True, text=True)
    if tool.returncode != 0 or not re.fullmatch(r'(?:tauri-cli|cargo-tauri) 2\.12\.1\s*', tool.stdout):
        raise ValueError('Signed packaging requires the verified official tauri-cli 2.12.1 signer')
    result = subprocess.run([str(signer), 'signer', 'sign', '--private-key-path', str(key),
                             '--app-version', version, str(archive)], env=env,
                            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if result.returncode != 0 or not Path(str(archive) + '.sig').is_file():
        raise ValueError('Official Tauri signer failed; no signed distribution was produced')


def build_start(output):
    packaging.write_json(output, {'version': packaging.versions.check(REPO),
                                  'source': packaging.source_snapshot(REPO)})


def build_finish(start, app, output):
    before = json.loads(start.read_text())
    if before['source'] != packaging.source_snapshot(REPO) or before['version'] != packaging.versions.check(REPO):
        raise ValueError('Source changed during the build; no verified build receipt produced')
    packaging.write_json(output, {'schema': 1, **before, 'bundle': packaging.bundle_entries(app),
                                  'profile': 'debug', 'apple_signing': 'ad-hoc', 'notarized': False,
                                  'build_command': 'cargo build --offline --locked --jobs 2 -p uploadimg-desktop',
                                  'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip()})


def package(output, signer, key, receipt_path):
    root = REPO / 'desktop'
    version = packaging.versions.check(REPO)
    app = root / '.build/UploadIMG.app'
    guard_output(root / '.build/distribution', output)
    receipt = json.loads(receipt_path.read_text())
    validate_build_receipt(receipt, REPO, app, version)
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    packaging.validate_info(info, plistlib.loads((root / 'src-tauri/Info.plist').read_bytes()), version)
    if packaging.digest(app / 'Contents/Resources/icon.icns') != packaging.digest(root / 'src-tauri/icons/icon.icns'):
        raise ValueError('Bundle icon differs from current source')
    packaging.inspect_binary(app / 'Contents/MacOS/UploadimgDesktop', True)
    packaging.inspect_binary(app / 'Contents/Frameworks/libUploadimgNative.dylib', False)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    # The reservation is atomic and deliberately remains after failure for inspection.
    output.mkdir(parents=True)
    with tempfile.TemporaryDirectory(prefix='.updater-', dir=output) as work_name:
        work = Path(work_name)
        archive = work / f'UploadIMG-{version}-macos-arm64.app.tar.gz'
        create_archive(app, archive)
        verify_archive(archive, app, work / 'extracted')
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(work / 'extracted/UploadIMG.app')], check=True)
        sign_archive(archive, signer, key, version)
        signature_path = Path(str(archive) + '.sig')
        public_key = (root / 'updater-public-key.txt').read_text().strip()
        manifest = make_manifest(version, signature_path.read_text(), public_key)
        channel = 'beta' if '-' in version.split('+')[0] else 'stable'
        feed = work / f'{channel}.json'
        packaging.write_json(feed, manifest)
        validate_build_receipt(receipt, REPO, app, version)
        provenance = work / 'updater-provenance.json'
        packaging.write_json(provenance, {'version': version, 'build_receipt': receipt,
            'compiled_source_commit': None if receipt['source']['git_dirty'] else receipt['source']['git_head'],
            'build_origin': 'Observed successful build.sh with unchanged source snapshot and exact bundle manifest',
            'profile': 'debug', 'apple_signing': 'ad-hoc', 'notarized': False,
            'updater_signer': 'official tauri-cli 2.12.1',
            'public_key_sha256': hashlib.sha256(public_key.encode()).hexdigest(),
            'artifact': {'path': archive.name, 'sha256': packaging.digest(archive)},
            'verified': ['build-source receipt matches current source and bundle', 'arm64/dependency/rpath guards',
                         'archive layout, bytes, modes and internal symlinks', 'deep/strict Apple signature after extraction',
                         'official signer completed; signature format/key ID/exact version match'],
            'not_run': ['independent cryptographic verification', 'installed-app upgrade/recovery',
                        'download quarantine/Gatekeeper', 'workflow execution', 'remote publication']})
        artifacts = [archive, signature_path, feed, provenance]
        checksum = work / 'UPDATER-SHA256SUMS'
        checksum.write_text(''.join(f'{packaging.digest(path)}  {path.name}\n' for path in artifacts))
        for path in [*artifacts, checksum]:
            os.link(path, output / path.name)
    print(f'PASS: updater archive layout/extraction, official signing and manifest publication guards: {output}')
    print('NOT_RUN: independent crypto verification, installed upgrade/recovery, CI, publication/downloads')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    start = commands.add_parser('build-start')
    start.add_argument('output', type=Path)
    finish = commands.add_parser('build-finish')
    for name in ('start', 'app', 'output'):
        finish.add_argument(name, type=Path)
    release = commands.add_parser('package')
    release.add_argument('--output', type=Path)
    release.add_argument('--signer', type=Path, default=REPO / 'desktop/.build/tools/tauri-cli-2.12.1/bin/cargo-tauri')
    release.add_argument('--key-file', type=Path, required=True)
    release.add_argument('--build-receipt', type=Path, default=REPO / 'desktop/.build/build-source.json')
    args = parser.parse_args()
    if args.command == 'build-start':
        build_start(args.output)
    elif args.command == 'build-finish':
        build_finish(args.start, args.app, args.output)
    else:
        output = args.output or REPO / 'desktop/.build/distribution' / (packaging.versions.check(REPO) + '-updater')
        package(output, args.signer, args.key_file, args.build_receipt)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError, tarfile.TarError) as error:
        raise SystemExit(f'Updater packaging failed: {error}')
