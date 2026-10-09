#!/usr/bin/env python3
"""Check distribution SemVer, numeric application versions, changelog and tag."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import re
import sys
import tomllib

CARGO_PATHS = ("desktop/crates/core/Cargo.toml", "desktop/src-tauri/Cargo.toml")
TAURI_PATH = "desktop/src-tauri/tauri.conf.json"
PLIST_PATH = "desktop/src-tauri/Info.plist"
SEMVER = re.compile(
    r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
)


def parse_version(version):
    match = SEMVER.fullmatch(version)
    if not match:
        raise ValueError(f"VERSION is not standard SemVer: {version!r}")
    for identifier in (match[4] or "").split("."):
        if identifier.isdigit() and len(identifier) > 1 and identifier.startswith("0"):
            raise ValueError("Numeric prerelease identifiers cannot have leading zeroes")
    return ".".join(match.group(1, 2, 3))


def check(root, tag=None):
    version = (root / "VERSION").read_text(encoding="utf-8").strip()
    base = parse_version(version)
    versions = {}
    for path in CARGO_PATHS:
        with (root / path).open("rb") as source:
            versions[path] = tomllib.load(source)["package"]["version"]
    versions[TAURI_PATH] = json.loads((root / TAURI_PATH).read_text())["version"]
    with (root / PLIST_PATH).open("rb") as source:
        versions[PLIST_PATH] = plistlib.load(source)["CFBundleShortVersionString"]
    for path, value in versions.items():
        if value != base:
            raise ValueError(f"{path}: expected {base}, found {value!r}")
    changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    if not re.search(r"^## \[" + re.escape(version) + r"\](?:\s+-\s+[^\n]+)?$", changelog, re.MULTILINE):
        raise ValueError(f"CHANGELOG.md needs an exact ## [{version}] entry")
    if tag is not None and tag != "v" + version:
        raise ValueError(f"Release tag must be v{version}; found {tag!r}")
    return version


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--tag", default=os.environ.get("RELEASE_TAG"), help="Exact tag; optional RELEASE_TAG environment fallback")
    arguments = parser.parse_args()
    try:
        version = check(arguments.root, arguments.tag)
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Version check failed: {error}", file=sys.stderr)
        return 1
    print(f"Version contract OK: {version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
