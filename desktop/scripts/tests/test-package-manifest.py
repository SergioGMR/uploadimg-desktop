"""Pure packaging guards; no disk images, signing, builds or app launches."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

script = Path(__file__).resolve().parents[1] / 'package-manifest.py'
spec = importlib.util.spec_from_file_location('package_manifest', script)
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)

class PackagingGuards(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.info = {'CFBundleIdentifier': 'local.uploadimg.desktop.m1',
                     'CFBundleShortVersionString': '0.1.0', 'CFBundleVersion': '2',
                     'CFBundleName': 'UploadIMG', 'CFBundleDisplayName': 'UploadIMG',
                     'CFBundleExecutable': 'UploadimgDesktop', 'CFBundleIconFile': 'icon.icns',
                     'LSMinimumSystemVersion': '15.4'}

    def test_beta2_identity_and_stale_build(self):
        packaging.validate_info(self.info, self.info, '0.1.0-beta.2')
        for key, value in [('CFBundleVersion', '1'), ('CFBundleName', 'Uploadimg Desktop'),
                           ('CFBundleShortVersionString', '0.2.0')]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                packaging.validate_info(dict(self.info, **{key: value}), self.info, '0.1.0-beta.2')

    def test_output_is_local_and_never_overwrites(self):
        base = self.root / 'distribution'
        output = base / '0.1.0-beta.2'
        packaging.guard_output(base, output, 'UploadIMG-0.1.0-beta.2-macos-arm64.dmg')
        output.mkdir(parents=True)
        (output / 'SHA256SUMS').write_text('existing')
        with self.assertRaises(ValueError):
            packaging.guard_output(base, output, 'UploadIMG-0.1.0-beta.2-macos-arm64.dmg')
        with self.assertRaises(ValueError):
            packaging.guard_output(base, self.root / 'outside', 'new.dmg')

    def test_bundle_manifest_detects_bytes_modes_and_preserves_internal_symlinks(self):
        bundle = self.root / 'UploadIMG.app'
        bundle.mkdir()
        resource = bundle / 'icon.icns'
        resource.write_bytes(b'icon-one')
        before = packaging.bundle_entries(bundle)
        resource.write_bytes(b'icon-two')
        self.assertNotEqual(before, packaging.bundle_entries(bundle))
        (bundle / 'link').symlink_to('icon.icns')
        self.assertEqual(packaging.bundle_entries(bundle)['link']['symlink'], 'icon.icns')
        resource.chmod(0o755)
        self.assertNotEqual(before, packaging.bundle_entries(bundle))
        (bundle / 'outside').symlink_to('../outside')
        with self.assertRaises(ValueError):
            packaging.bundle_entries(bundle)

    def test_attachment_uses_exact_device_and_mount(self):
        mount = self.root / 'mount'
        entries = [{'dev-entry': '/dev/disk42'},
                   {'dev-entry': '/dev/disk42s1', 'mount-point': str(mount)}]
        self.assertEqual(packaging.attachment({'system-entities': entries}, mount), '/dev/disk42')
        with self.assertRaises(ValueError):
            packaging.attachment({'system-entities': entries}, self.root / 'other')
        with self.assertRaises(ValueError):
            packaging.attachment({'system-entities': entries + [{'dev-entry': '/dev/disk43'}]}, mount)

    def test_source_receipt_retains_dirty_head_and_detects_changes(self):
        source = self.root / 'source.swift'
        source.write_text('before')
        def git(command):
            if 'ls-files' in command: return b'source.swift\0'
            if 'status' in command: return b' M source.swift\0'
            return b'0123456789abcdef\n'
        with patch.object(packaging.subprocess, 'check_output', side_effect=git):
            before = packaging.source_snapshot(self.root)
            self.assertTrue(before['git_dirty'])
            self.assertEqual(before['git_head'], '0123456789abcdef')
            source.write_text('after')
            self.assertNotEqual(before['sha256'], packaging.source_snapshot(self.root)['sha256'])

    def test_exclusive_provenance_write_preserves_existing_bytes(self):
        target = self.root / 'provenance.json'
        packaging.write_json(target, {'first': True})
        before = target.read_bytes()
        with self.assertRaises(FileExistsError):
            packaging.write_json(target, {'second': True})
        self.assertEqual(before, target.read_bytes())

if __name__ == '__main__':
    unittest.main()
