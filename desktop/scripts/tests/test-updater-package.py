"""Isolated archive/receipt guards; signature fixtures test format, never cryptography."""
import base64
import importlib.util
import json
from pathlib import Path
import plistlib
import tarfile
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

script = Path(__file__).resolve().parents[1] / 'package-updater.py'
spec = importlib.util.spec_from_file_location('updater_package', script)
updater = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updater)


class UpdaterPackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.app = self.root / 'UploadIMG.app'
        (self.app / 'Contents/MacOS').mkdir(parents=True)
        (self.app / 'Contents/Frameworks/Native.framework/Versions/A').mkdir(parents=True)
        (self.app / 'Contents/Resources').mkdir()
        binary = self.app / 'Contents/MacOS/UploadimgDesktop'
        binary.write_bytes(b'executable-fixture')
        binary.chmod(0o755)
        (self.app / 'Contents/Frameworks/Native.framework/Versions/A/Native').write_bytes(b'native-fixture')
        (self.app / 'Contents/Frameworks/Native.framework/Versions/Current').symlink_to('A')
        (self.app / 'Contents/Resources/icon.icns').write_bytes(b'icon-fixture')
        (self.app / 'Contents/Info.plist').write_bytes(plistlib.dumps({'CFBundleVersion': '3'}))
        self.version = '0.1.0-beta.3'
        self.key_id = b'12345678'
        self.public = base64.b64encode(b'untrusted comment: fixture\n' + base64.b64encode(b'Ed' + self.key_id + bytes(32)) + b'\n').decode()

    def signature(self, version=None):
        # Structurally valid all-zero signature: intentionally NOT cryptographically valid.
        data = b'untrusted comment: format fixture only\n' + base64.b64encode(b'ED' + self.key_id + bytes(64))
        data += f'\ntrusted comment: timestamp:1\tfile:fixture\tversion:{version or self.version}\n'.encode()
        data += base64.b64encode(bytes(64)) + b'\n'
        return base64.b64encode(data).decode()

    def test_archive_preserves_contents_permissions_and_internal_symlinks(self):
        (self.app / 'Contents/Resources').chmod(0o700)
        (self.app / 'Contents/MacOS/UploadimgDesktop').chmod(0o775)
        archive = self.root / 'UploadIMG.app.tar.gz'
        updater.create_archive(self.app, archive)
        updater.verify_archive(archive, self.app, self.root / 'extracted')
        self.assertEqual(updater.packaging.bundle_entries(self.app),
                         updater.packaging.bundle_entries(self.root / 'extracted/UploadIMG.app'))

    def test_archive_rejects_wrong_layout_traversal_and_overwrite(self):
        archive = self.root / 'bad.tar.gz'
        for name in ['Wrong.app/file', 'UploadIMG.app/../escape']:
            with self.subTest(name=name):
                with tarfile.open(archive, 'w:gz') as target:
                    target.addfile(tarfile.TarInfo(name))
                with self.assertRaises(ValueError):
                    updater.verify_archive(archive, self.app, self.root / 'extracted')
        with self.assertRaises(FileExistsError):
            updater.create_archive(self.app, archive)

    def test_manifest_contains_full_version_signature_contents_and_exact_release_url(self):
        signature = self.signature()
        feed = updater.make_manifest(self.version, signature, self.public)
        self.assertEqual(feed['version'], self.version)
        self.assertEqual(list(feed['platforms']), ['darwin-aarch64'])
        self.assertEqual(feed['platforms']['darwin-aarch64']['signature'], signature)
        self.assertEqual(feed['platforms']['darwin-aarch64']['url'],
                         'https://github.com/SergioGMR/uploadimg-desktop/releases/download/v0.1.0-beta.3/UploadIMG-0.1.0-beta.3-macos-arm64.app.tar.gz')
        self.assertLessEqual(len(json.dumps(feed).encode()), 65536)

    def test_manifest_rejects_missing_invalid_wrong_key_or_wrong_signed_version(self):
        for signature in ['', 'https://example.com/update.sig', 'a' * 70000, self.signature('0.1.0-beta.2')]:
            with self.subTest(signature=signature[:32]), self.assertRaises(ValueError):
                updater.make_manifest(self.version, signature, self.public)
        other = self.public.replace(self.public[0], '!')
        with self.assertRaises(ValueError):
            updater.make_manifest(self.version, self.signature(), other)
        self.key_id = b'87654321'
        with self.assertRaises(ValueError):
            updater.make_manifest(self.version, self.signature(), self.public)

    def test_build_receipt_requires_exact_source_version_and_bundle(self):
        source = {'git_head': 'abc', 'git_dirty': True, 'sha256': 'source-digest'}
        receipt = {'schema': 1, 'version': self.version, 'source': source,
                   'bundle': updater.packaging.bundle_entries(self.app)}
        with patch.object(updater.packaging, 'source_snapshot', return_value=source):
            updater.validate_build_receipt(receipt, self.root, self.app, self.version)
            for wrong in [dict(receipt, version='0.1.0-beta.2'), dict(receipt, source={}), dict(receipt, bundle={})]:
                with self.assertRaises(ValueError):
                    updater.validate_build_receipt(wrong, self.root, self.app, self.version)

    def test_signer_missing_key_or_failure_never_claims_signed_artifact(self):
        archive = self.root / 'artifact.app.tar.gz'
        archive.write_bytes(b'fixture')
        missing = self.root / 'missing-key'
        with self.assertRaises(ValueError):
            updater.sign_archive(archive, self.root / 'signer', missing, self.version)
        key = self.root / 'fixture-key'
        key.write_text('not a real key')
        key.chmod(0o600)
        with patch.object(updater.subprocess, 'run') as run:
            run.side_effect = [SimpleNamespace(returncode=0, stdout='tauri-cli 2.12.1\n'),
                               SimpleNamespace(returncode=1)]
            with self.assertRaises(ValueError):
                updater.sign_archive(archive, self.root / 'signer', key, self.version)
            self.assertEqual(run.call_count, 2)
        self.assertFalse(Path(str(archive) + '.sig').exists())

    def test_output_refuses_existing_or_outside_distribution(self):
        base = self.root / 'distribution'
        updater.guard_output(base, base / self.version)
        (base / self.version).mkdir(parents=True)
        with self.assertRaises(ValueError):
            updater.guard_output(base, base / self.version)
        with self.assertRaises(ValueError):
            updater.guard_output(base, self.root / 'outside')

    def test_wrong_signer_version_is_rejected_before_signing(self):
        archive = self.root / 'artifact.app.tar.gz'
        archive.write_bytes(b'fixture')
        key = self.root / 'fixture-key'
        key.write_text('fixture-only')
        key.chmod(0o600)
        with patch.object(updater.subprocess, 'run') as run:
            run.return_value.returncode = 0
            run.return_value.stdout = 'tauri-cli 9.9.9'
            with self.assertRaises(ValueError):
                updater.sign_archive(archive, self.root / 'signer', key, self.version)
            self.assertEqual(run.call_args_list[0].args[0], [str(self.root / 'signer'), '--version'])
            self.assertEqual(run.call_count, 1)

    def test_build_finish_writes_only_unchanged_source_and_records_dirty_identity(self):
        start = self.root / 'start.json'
        output = self.root / 'build-source.json'
        source = {'git_head': 'abc', 'git_dirty': True, 'sha256': 'exact-source'}
        start.write_text(json.dumps({'version': self.version, 'source': source}))
        with patch.object(updater.packaging, 'source_snapshot', return_value={'sha256': 'changed'}):
            with self.assertRaises(ValueError):
                updater.build_finish(start, self.app, output)
        self.assertFalse(output.exists())
        with patch.object(updater.packaging, 'source_snapshot', return_value=source), \
             patch.object(updater.packaging.versions, 'check', return_value=self.version), \
             patch.object(updater.subprocess, 'check_output', return_value='rustc 1.92.0\n'):
            updater.build_finish(start, self.app, output)
        receipt = json.loads(output.read_text())
        self.assertTrue(receipt['source']['git_dirty'])
        self.assertEqual(receipt['source']['sha256'], 'exact-source')
        self.assertEqual(receipt['bundle'], updater.packaging.bundle_entries(self.app))


if __name__ == '__main__':
    unittest.main()
