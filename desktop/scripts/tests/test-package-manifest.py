"""Pure packaging guards; no disk images, signing, builds or app launches."""
import contextlib
import importlib.util
import json
from pathlib import Path
import plistlib
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

script = Path(__file__).resolve().parents[1] / 'package-manifest.py'
spec = importlib.util.spec_from_file_location('package_manifest', script)
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)

LEGAL = ['LICENSE', 'COPYRIGHT', 'THIRD-PARTY-NOTICES']


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

    def legal_bundle(self):
        app = self.root / 'UploadIMG.app'
        resources = app / 'Contents/Resources'
        resources.mkdir(parents=True)
        for name in LEGAL:
            (self.root / name).write_text('source legal fixture: ' + name)
            shutil.copyfile(self.root / name, resources / name)
        (resources / 'icon.icns').write_bytes(b'icon')
        (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(self.info))
        desktop = self.root / 'desktop'
        (desktop / 'src-tauri/icons').mkdir(parents=True)
        (desktop / 'src-tauri/Info.plist').write_bytes(plistlib.dumps(self.info))
        (desktop / 'src-tauri/icons/icon.icns').write_bytes(b'icon')
        (desktop / 'distribution').mkdir()
        return app

    def run_main(self, *args):
        with patch.object(packaging, 'REPO', self.root), \
             patch.object(sys, 'argv', ['package-manifest.py', *map(str, args)]), \
             patch.object(packaging.versions, 'check', return_value='0.1.0-beta.2'), \
             patch.object(packaging, 'source_snapshot', return_value={}), \
             patch.object(packaging, 'inspect_binary'):
            packaging.main()

    @contextlib.contextmanager
    def tampered(self, bundles, name, value):
        """Remove (None) or alter a legal text in each bundle; always restore it from the source copy."""
        try:
            for bundle in bundles:
                resource = bundle / 'Contents/Resources' / name
                if value is None:
                    resource.unlink()
                else:
                    resource.write_bytes(value)
            yield
        finally:
            for bundle in bundles:
                shutil.copyfile(self.root / name, bundle / 'Contents/Resources' / name)

    def test_capture_rejects_missing_or_changed_legal_resources_before_receipt(self):
        app = self.legal_bundle()
        for name in LEGAL:
            for value in [None, b'altered legal text']:
                with self.subTest(name=name, value=value), self.tampered([app], name, value):
                    manifest = self.root / f'rejected-{name}-{value is None}.json'
                    with self.assertRaises(ValueError):
                        self.run_main('capture', app, manifest)
                    self.assertFalse(manifest.exists())

    @contextlib.contextmanager
    def source_tampered(self, name, value):
        """Remove (None) or alter a source legal text; always restore it."""
        source = self.root / name
        saved = source.read_bytes()
        try:
            if value is None:
                source.unlink()
            else:
                source.write_bytes(value)
            yield
        finally:
            source.write_bytes(saved)

    def test_capture_rejects_missing_source_legal_text(self):
        app = self.legal_bundle()
        for name in LEGAL:
            with self.subTest(name=name), self.source_tampered(name, None):
                manifest = self.root / f'no-source-{name}.json'
                with self.assertRaises(ValueError):
                    self.run_main('capture', app, manifest)
                self.assertFalse(manifest.exists())

    def test_capture_rejects_symlinked_legal_text_even_with_identical_bytes(self):
        app = self.legal_bundle()
        for name in LEGAL:
            twin = self.root / f'twin-{name}'
            twin.write_bytes((self.root / name).read_bytes())
            with self.subTest(side='bundle', name=name):
                resource = app / 'Contents/Resources' / name
                resource.unlink()
                try:
                    resource.symlink_to(twin)
                    manifest = self.root / f'link-bundle-{name}.json'
                    with self.assertRaises(ValueError):
                        self.run_main('capture', app, manifest)
                    self.assertFalse(manifest.exists())
                finally:
                    resource.unlink()
                    shutil.copyfile(self.root / name, resource)
            with self.subTest(side='source', name=name):
                source = self.root / name
                saved = source.read_bytes()
                source.unlink()
                try:
                    source.symlink_to(twin)
                    manifest = self.root / f'link-source-{name}.json'
                    with self.assertRaises(ValueError):
                        self.run_main('capture', app, manifest)
                    self.assertFalse(manifest.exists())
                finally:
                    source.unlink()
                    source.write_bytes(saved)

    def mounted_dmg(self):
        app = self.legal_bundle()
        mount = self.root / 'mount'
        mount.mkdir()
        mounted_app = mount / 'UploadIMG.app'
        shutil.copytree(app, mounted_app)
        (mount / 'Applications').symlink_to('/Applications')
        for source, target in [('README.md', 'README.txt'), ('release-notes.md', 'RELEASE-NOTES.txt')]:
            (self.root / 'desktop/distribution' / source).write_text(source)
            (mount / target).write_text(source)
        return app, mount, mounted_app

    def test_mounted_dmg_fixture_preserves_legal_texts(self):
        app, mount, mounted_app = self.mounted_dmg()
        manifest = self.root / 'manifest.json'
        self.run_main('capture', app, manifest)
        self.run_main('compare', app, mount, manifest)
        for name in LEGAL:
            self.assertEqual((mounted_app / 'Contents/Resources' / name).read_bytes(),
                             (self.root / name).read_bytes())

    def test_compare_rejects_altered_or_missing_legal_text_in_mounted_bundle(self):
        app, mount, mounted_app = self.mounted_dmg()
        manifest = self.root / 'manifest.json'
        self.run_main('capture', app, manifest)
        for name in LEGAL:
            for value in [None, b'altered legal text']:
                with self.subTest(name=name, value=value), self.tampered([mounted_app], name, value):
                    with self.assertRaises(ValueError):
                        self.run_main('compare', app, mount, manifest)

    def test_compare_rejects_sealed_omissions_in_both_bundles(self):
        app, mount, mounted_app = self.mounted_dmg()
        for name in LEGAL:
            for value in [None, b'sealed altered legal text']:
                with self.subTest(name=name, value=value), self.tampered([app, mounted_app], name, value):
                    # A manifest that sealed the tampered bundle must still be rejected.
                    manifest = self.root / f'sealed-{name}-{value is None}.json'
                    manifest.write_text(json.dumps({'source_at_packaging': {},
                                                   'bundle': packaging.bundle_entries(app)}))
                    with self.assertRaises(ValueError):
                        self.run_main('compare', app, mount, manifest)

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
