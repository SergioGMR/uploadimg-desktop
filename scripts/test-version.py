#!/usr/bin/env python3
"""Version contract tests; isolated fixtures never edit application manifests."""
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("check-version.py")
SPEC = importlib.util.spec_from_file_location("version_check", SCRIPT)
VERSION_CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERSION_CHECK)


class VersionContract(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.version = "0.1.0-beta.1"
        self.write("VERSION", self.version + "\n")
        self.write("CHANGELOG.md", f"# Changelog\n\n## [{self.version}]\n")
        for path in VERSION_CHECK.CARGO_PATHS:
            self.write(path, '[package]\nname = "fixture"\nversion = "0.1.0"\n')
        self.write(VERSION_CHECK.TAURI_PATH, json.dumps({"version": "0.1.0"}))
        target = self.root / VERSION_CHECK.PLIST_PATH
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(plistlib.dumps({"CFBundleShortVersionString": "0.1.0"}))

    def write(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    def test_standard_semver_and_numeric_base(self):
        for version in ("0.1.0", "1.2.3-alpha.0", "1.2.3-rc.1+build.007", "10.20.30+007"):
            with self.subTest(version=version):
                self.assertEqual(VERSION_CHECK.parse_version(version), version.split("-")[0].split("+")[0])

    def test_rejects_nonstandard_versions(self):
        for version in ("v1.2.3", "01.2.3", "1.02.3", "1.2.03", "1.2", "1.2.3-01", "1.2.3-a.00", "1.2.3-", "1.2.3+", "1.2.3-a..b", "1.2.3+a_b", " 1.2.3", "1.2.3\n", "١.2.3"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                VERSION_CHECK.parse_version(version)

    def test_prerelease_manifest_base_and_exact_tag(self):
        self.assertEqual(VERSION_CHECK.check(self.root, "v" + self.version), self.version)
        for tag in (self.version, "v0.1.0", "v0.1.0-beta.2", "refs/tags/v" + self.version):
            with self.subTest(tag=tag), self.assertRaisesRegex(ValueError, "tag"):
                VERSION_CHECK.check(self.root, tag)

    def test_detects_drift_in_each_cargo_manifest(self):
        for path in VERSION_CHECK.CARGO_PATHS:
            self.write(path, '[package]\nversion = "0.2.0"\n')
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, "0.1.0"):
                VERSION_CHECK.check(self.root)
            self.write(path, '[package]\nversion = "0.1.0"\n')

    def test_detects_tauri_and_plist_drift(self):
        self.write(VERSION_CHECK.TAURI_PATH, '{"version": "0.2.0"}')
        with self.assertRaises(ValueError):
            VERSION_CHECK.check(self.root)
        self.write(VERSION_CHECK.TAURI_PATH, '{"version": "0.1.0"}')
        (self.root / VERSION_CHECK.PLIST_PATH).write_bytes(plistlib.dumps({"CFBundleShortVersionString": "0.2.0"}))
        with self.assertRaises(ValueError):
            VERSION_CHECK.check(self.root)

    def test_requires_exact_changelog_heading(self):
        for content in (f"Mention {self.version}\n", "## [0.1.0-beta.10]\n"):
            self.write("CHANGELOG.md", content)
            with self.assertRaisesRegex(ValueError, "CHANGELOG"):
                VERSION_CHECK.check(self.root)

    def test_build_metadata_participates_in_tag_and_changelog(self):
        version = self.version + "+build.007"
        self.write("VERSION", version + "\n")
        self.write("CHANGELOG.md", f"## [{version}]\n")
        self.assertEqual(VERSION_CHECK.check(self.root, "v" + version), version)

    def test_cli_explicit_environment_tag_and_failure_exit(self):
        environment = dict(os.environ, RELEASE_TAG="v0.1.0-beta.2")
        result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(self.root)], env=environment, capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertIn("tag", result.stderr)
        result = subprocess.run([sys.executable, str(SCRIPT), "--root", str(self.root), "--tag", "v" + self.version], env=environment, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
