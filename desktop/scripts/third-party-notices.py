#!/usr/bin/env python3
"""Generate or check THIRD-PARTY-NOTICES for the macOS application.

    third-party-notices.py generate   write the root THIRD-PARTY-NOTICES (nothing else)
    third-party-notices.py --check    fail if the committed file differs from the generated one

The crate set comes from `cargo metadata` for Cargo.lock on aarch64-apple-darwin, offline.
License texts are the verbatim files shipped in each crate directory (stdlib only, no network).
This lists what ships in the binary; it is not a legal audit of those licenses.
"""
import difflib
import json
import os
from pathlib import Path
import subprocess
import sys

REPO = Path(__file__).resolve().parents[2]
WORKSPACE_DIR = 'desktop'
OUTPUT = 'THIRD-PARTY-NOTICES'
APPLICATION = 'uploadimg-desktop'
TARGET = 'aarch64-apple-darwin'
LICENSE_PREFIXES = ('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT', 'UNLICENSE',
                    'THIRD_PARTY', 'THIRD-PARTY')
# Source and build files that merely share a license-like name (for example objc2-foundation's
# src/copying.rs). Only code extensions are excluded: crates do ship LICENSE.md, NOTICE.md and
# LICENSE.txt, which must stay. Verified against the current graph: this drops only copying.rs.
CODE_EXTENSIONS = frozenset({
    '.rs', '.c', '.h', '.cc', '.cpp', '.cxx', '.hpp', '.hh', '.m', '.mm', '.s', '.asm', '.swift',
    '.py', '.pl', '.rb', '.sh', '.js', '.mjs', '.ts', '.go', '.java', '.toml', '.json', '.yml',
    '.yaml', '.lock', '.in', '.cmake', '.def'})
NO_LICENSE_TITLE = 'Crates without a license file in the published package'
GENERATE_COMMAND = 'python3 desktop/scripts/third-party-notices.py generate'
RULE = '=' * 78
THIN_RULE = '-' * 78


def load_metadata(repo):
    command = ['cargo', 'metadata', '--format-version', '1', '--locked', '--offline',
               '--filter-platform', TARGET, '--manifest-path', str(repo / WORKSPACE_DIR / 'Cargo.toml')]
    result = subprocess.run(command, check=True, capture_output=True, text=True, encoding='utf-8')
    return json.loads(result.stdout)


def is_proc_macro(package):
    return any('proc-macro' in target['kind'] for target in package['targets'])


def linked_packages(metadata):
    """Crates linked into the application: normal dependency edges only, sorted by name/version.

    Dev and build dependency edges are not followed: they never ship in the binary. Proc-macro
    crates run inside the compiler and are not linked either, so they and anything reachable
    only through them are left out. Workspace members are the project's own GPL crates; they
    are traversed but not listed. Vendored path dependencies (the patched GLib) are listed.
    """
    packages = {package['id']: package for package in metadata['packages']}
    members = set(metadata['workspace_members'])
    nodes = {node['id']: node['deps'] for node in metadata['resolve']['nodes']}
    roots = [pid for pid in members if pid in packages and packages[pid]['name'] == APPLICATION]
    if len(roots) != 1:
        raise ValueError(f'Expected exactly one workspace package named {APPLICATION}')
    seen, pending = {roots[0]}, [roots[0]]
    while pending:
        for dep in nodes.get(pending.pop(), []):
            child = dep['pkg']
            if child in seen or not any(kind['kind'] is None for kind in dep['dep_kinds']):
                continue
            if is_proc_macro(packages[child]):
                continue
            seen.add(child)
            pending.append(child)
    return sorted((packages[pid] for pid in seen - members), key=lambda p: (p['name'], p['version']))


def normalize(raw, name):
    """Decode strictly as UTF-8 (BOM dropped); CRLF to LF and trailing blank lines trimmed."""
    try:
        text = raw.decode('utf-8-sig')
    except UnicodeDecodeError as error:
        raise ValueError(f'License text {name} is not valid UTF-8') from error
    return text.replace('\r\n', '\n').rstrip('\n')


def license_texts(package):
    """(crate-relative POSIX path, text) for the license_file field and license-like files at any depth.

    Nested files matter: ring ships third_party/fiat/LICENSE and regex-syntax a Unicode license
    under src/. Symlinks (files or directories) are neither followed nor read, so nothing outside
    the crate directory can enter the notices; the license_file field must resolve inside it.
    """
    directory = Path(package['manifest_path']).parent
    names = set()
    if package.get('license_file'):
        names.add(Path(package['license_file']).as_posix())
    for current, subdirectories, files in os.walk(directory, followlinks=False):
        subdirectories.sort()
        for filename in files:
            path = Path(current, filename)
            if (filename.upper().startswith(LICENSE_PREFIXES) and not path.is_symlink()
                    and path.suffix.lower() not in CODE_EXTENSIONS):
                names.add(path.relative_to(directory).as_posix())
    found = []
    for name in sorted(names):
        path = directory / name
        if not path.is_file():
            continue
        if not path.resolve().is_relative_to(directory.resolve()):
            raise ValueError(f"License file of {package['name']} {package['version']} is outside the crate")
        found.append((name, normalize(path.read_bytes(), f"{package['name']}/{name}")))
    return found


def source_label(package, workspace_root):
    source = package.get('source')
    if source is None:
        directory = Path(package['manifest_path']).parent
        try:
            relative = directory.relative_to(workspace_root).as_posix()
        except ValueError as error:
            raise ValueError(f"Path dependency {package['name']} is outside the workspace") from error
        label = f'vendored in this repository: {WORKSPACE_DIR}/{relative}'
    elif 'crates.io' in source:
        label = 'crates.io'
    else:
        label = source
    if package.get('repository'):
        label += f"; repository {package['repository']}"
    return label


def declared_license(package):
    if package.get('license'):
        return package['license']
    if package.get('license_file'):
        return 'declared through license-file (see the text below)'
    return 'not declared'


HEADER = f"""THIRD-PARTY NOTICES

UploadIMG Desktop's original material is licensed under GPL-3.0-only (see LICENSE and
COPYRIGHT). The Rust crates below are part of the macOS application's dependency graph
(target {TARGET}) and keep their own licenses and copyright notices, reproduced here
from the files shipped in each crate. Exact versions come from desktop/Cargo.lock; their
sources are pinned there by checksums on crates.io.

Scope: normal (non-dev, non-build) dependencies of the application for that target,
excluding proc-macro crates, which run inside the compiler and are not linked into the
binary. Cargo unifies features across build and normal use, so the list can include
compile-time helper crates; it is a superset rather than a minimal set. The vendored GLib
(desktop/vendor/glib-0.18.5, MIT, with its own LICENSE and COPYRIGHT) is not in the macOS
dependency graph and is therefore not listed. Native system libraries and non-Rust
components are not covered. This faithfully reproduces what the crates declare and ship;
it is not an exhaustive legal audit.

Generated by desktop/scripts/third-party-notices.py from `cargo metadata`; do not edit by hand.
Regenerate with: {GENERATE_COMMAND}"""


def render(metadata):
    workspace_root = Path(metadata['workspace_root'])
    packages = linked_packages(metadata)
    entries = []
    for package in packages:
        key = (package['name'], package['version'])
        entries.append((key, package, license_texts(package)))

    # Identical texts are printed once; numbering follows the sorted first user of each text.
    users = {}
    for key, package, files in entries:
        for name, text in files:
            users.setdefault(text, []).append((key[0], key[1], name))
    ordered = sorted(users, key=lambda text: min(users[text]))
    number = {text: index for index, text in enumerate(ordered, 1)}

    listing, bare = [], []
    for key, package, files in entries:
        lines = [f'{key[0]} {key[1]}',
                 f"  License: {declared_license(package)}",
                 f'  Source: {source_label(package, workspace_root)}']
        if files:
            refs = '; '.join(f'{name} [{number[text]}]' for name, text in files)
            lines.append(f'  License files: {refs}')
        else:
            lines.append('  License files: none (the published crate ships no license file)')
            bare.append(f"{key[0]} {key[1]}: declared {package.get('license') or 'no license expression'}")
        listing.append('\n'.join(lines))

    sections = [HEADER,
                f'Crates ({len(entries)})\n{RULE}\n' + '\n\n'.join(listing),
                f'{NO_LICENSE_TITLE} ({len(bare)})\n{RULE}\n'
                + ('For these crates the published crate ships no license file, so no license text is\n'
                   'reproduced; only the license expression declared in the crate metadata is shown.\n\n'
                   + '\n'.join(bare) if bare else 'None.')]
    texts = []
    for text in ordered:
        names = '\n'.join(f'  {crate} {version} ({name})' for crate, version, name in sorted(users[text]))
        texts.append(f'[{number[text]}] License text\n\nUsed by:\n{names}\n{THIN_RULE}\n{text}')
    sections.append(f'License texts ({len(ordered)})\n{RULE}\n' + f'\n\n{RULE}\n'.join(texts))
    return '\n\n\n'.join(sections) + '\n'


def usage():
    print('usage: third-party-notices.py generate | --check', file=sys.stderr)
    raise SystemExit(2)


def main(argv=None, repo=REPO):
    argv = sys.argv[1:] if argv is None else argv
    if argv not in (['generate'], ['--check']):
        usage()
    try:
        expected = render(load_metadata(repo)).encode('utf-8')
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        detail = getattr(error, 'stderr', None) or error
        raise SystemExit(f'Cannot generate {OUTPUT}: {detail}') from error
    target = repo / OUTPUT
    if argv == ['generate']:
        if target.is_symlink() or (target.exists() and not target.is_file()):
            raise SystemExit(f'Refusing to write {OUTPUT}: it is not a regular file')
        if not target.exists() or target.read_bytes() != expected:
            target.write_bytes(expected)
            print(f'Wrote {OUTPUT}')
        else:
            print(f'{OUTPUT} is already current')
        return 0
    if not target.is_file() or target.is_symlink():
        print(f'{OUTPUT} is missing; run: {GENERATE_COMMAND}', file=sys.stderr)
        return 1
    actual = target.read_bytes()
    if actual != expected:
        diff = difflib.unified_diff(actual.decode('utf-8', 'replace').splitlines(),
                                    expected.decode('utf-8').splitlines(),
                                    'committed', 'generated', lineterm='', n=0)
        print(f'{OUTPUT} differs from the dependency set in Cargo.lock; run: {GENERATE_COMMAND}',
              file=sys.stderr)
        for line in list(diff)[:12]:
            print(line, file=sys.stderr)
        return 1
    print(f'{OUTPUT} is current')
    return 0


if __name__ == '__main__':
    sys.exit(main())
