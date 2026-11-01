"""THIRD-PARTY-NOTICES generator guards; fake cargo metadata and temp crates, no cargo or network."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import random
import sys
import tempfile
import unittest
from unittest.mock import patch

script = Path(__file__).resolve().parents[1] / 'third-party-notices.py'
spec = importlib.util.spec_from_file_location('third_party_notices', script)
notices = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notices)

REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
MIT = 'MIT fixture text\nSecond line\n'
APACHE = 'Apache fixture text\n'


class NoticesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.workspace = self.root / 'workspace'
        self.registry = self.root / 'registry'
        self.packages = {}
        self.edges = {}
        stdout = patch.object(sys, 'stdout', io.StringIO())
        stdout.start()
        self.addCleanup(stdout.stop)

    def pid(self, name, version='1.0.0'):
        return f'{REGISTRY}#{name}@{version}'

    def add(self, name, version='1.0.0', license='MIT', files=None, license_file=None, kinds=None,
            source=REGISTRY, directory=None, member=False, repository=None):
        directory = directory or self.registry / f'{name}-{version}'
        directory.mkdir(parents=True, exist_ok=True)
        for filename, text in (files or {}).items():
            path = directory / filename
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(text.encode())
        pid = self.pid(name, version) if not member else f'path+file://{directory}#{name}@{version}'
        self.packages[pid] = {
            'name': name, 'version': version, 'id': pid, 'license': license, 'license_file': license_file,
            'source': None if member or source is None else source, 'repository': repository,
            'manifest_path': str(directory / 'Cargo.toml'),
            'targets': [{'kind': kinds or ['lib']}], 'member': member}
        self.edges.setdefault(pid, [])
        return pid

    def link(self, parent, child, kinds=(None,)):
        self.edges[parent].append(
            {'name': 'dep', 'pkg': child, 'dep_kinds': [{'kind': kind, 'target': None} for kind in kinds]})

    def metadata(self):
        packages = [{k: v for k, v in package.items() if k != 'member'} for package in self.packages.values()]
        return {'workspace_root': str(self.workspace),
                'workspace_members': [pid for pid, package in self.packages.items() if package['member']],
                'packages': packages,
                'resolve': {'root': None, 'nodes': [
                    {'id': pid, 'deps': deps} for pid, deps in self.edges.items()]}}

    def graph(self):
        """app -> core(member) -> mit/apache crates; plus excluded dev/build/proc-macro/transitive ones."""
        app = self.add('uploadimg-desktop', license='GPL-3.0-only', member=True,
                       directory=self.workspace / 'src-tauri')
        core = self.add('uploadimg-core', license='GPL-3.0-only', member=True,
                        directory=self.workspace / 'crates/core')
        self.link(app, core)
        a = self.add('alpha', '1.2.0', 'MIT OR Apache-2.0',
                     {'LICENSE-MIT': MIT, 'LICENSE-APACHE': APACHE, 'README.md': 'readme'},
                     repository='https://example.test/alpha')
        b = self.add('beta', '0.3.0', 'MIT', {'license-mit': MIT, 'Notice': 'beta notice\n'})
        self.link(app, a)
        self.link(a, b)
        return app, a, b

    def render(self):
        return notices.render(self.metadata())

    def test_only_normal_edges_from_the_application_are_listed(self):
        app, a, b = self.graph()
        dev = self.add('devonly', files={'LICENSE': 'dev'})
        build = self.add('buildonly', files={'LICENSE': 'build'})
        macro = self.add('macro', files={'LICENSE': 'macro'}, kinds=['proc-macro'])
        behind = self.add('behindmacro', files={'LICENSE': 'behind'})
        both = self.add('both', files={'LICENSE': 'both'})
        self.link(app, dev, ['dev'])
        self.link(app, build, ['build'])
        self.link(app, macro)
        self.link(macro, behind)
        self.link(a, both, ['build', None])
        names = [package['name'] for package in notices.linked_packages(self.metadata())]
        self.assertEqual(names, ['alpha', 'beta', 'both'])

    def test_workspace_members_are_excluded_but_traversed(self):
        app, a, b = self.graph()
        core = next(pid for pid in self.packages if 'uploadimg-core' in pid)
        extra = self.add('viacore', files={'LICENSE': 'x'})
        self.link(core, extra)
        names = [package['name'] for package in notices.linked_packages(self.metadata())]
        self.assertEqual(names, ['alpha', 'beta', 'viacore'])

    def test_missing_application_package_is_an_error(self):
        self.graph()
        data = self.metadata()
        data['packages'] = [p for p in data['packages'] if p['name'] != 'uploadimg-desktop']
        with self.assertRaises(ValueError):
            notices.linked_packages(data)

    def test_license_files_are_found_case_insensitively_including_nested_ones(self):
        app, a, b = self.graph()
        extra = self.add('gamma', files={
            'LICENCE': 'l1', 'copying.txt': 'c1', 'UNLICENSE': 'u1', 'Copyright': 'cr',
            'LICENSE-MIT': 'm', 'notice.md': 'n', 'README.md': 'r', 'THIRD_PARTY': 'tp',
            'src/LICENSE': 'nested', 'deps/fiat/LICENSE': 'fiat',
            'src/unicode_tables/LICENSE-UNICODE': 'unicode', 'a/b/c/Notice.MD': 'deep'})
        self.link(a, extra)
        package = self.packages[extra]
        found = [name for name, _ in notices.license_texts(package)]
        self.assertNotIn('README.md', found)
        for expected in ['LICENCE', 'copying.txt', 'UNLICENSE', 'Copyright', 'LICENSE-MIT', 'notice.md',
                         'THIRD_PARTY', 'src/LICENSE', 'deps/fiat/LICENSE',
                         'src/unicode_tables/LICENSE-UNICODE', 'a/b/c/Notice.MD']:
            self.assertIn(expected, found)
        self.assertEqual(found, sorted(found))

    def test_source_files_named_like_licenses_are_not_license_texts(self):
        app, a, b = self.graph()
        extra = self.add('delta', files={
            'LICENSE': 'real', 'src/copying.rs': 'fn copy() {}', 'src/tests/copying.rs': 'fn t() {}',
            'src/license.c': 'int x;', 'include/notice.h': '#pragma once', 'tools/copyright.py': 'x=1',
            'asm/license.S': 'nop', 'Cargo-notice.toml': '', 'notice.json': '{}'})
        self.link(a, extra)
        found = [name for name, _ in notices.license_texts(self.packages[extra])]
        self.assertEqual(found, ['LICENSE'])

    def test_symlinks_are_neither_followed_nor_read(self):
        app, a, b = self.graph()
        outside = self.root / 'outside'
        outside.mkdir()
        (outside / 'LICENSE').write_text('outside secret')
        extra = self.add('linked', files={'LICENSE': 'real'})
        directory = self.registry / 'linked-1.0.0'
        (directory / 'LICENSE-LINK').symlink_to(outside / 'LICENSE')
        (directory / 'linkdir').symlink_to(outside)
        (directory / 'real').mkdir()
        (directory / 'real/NOTICE').symlink_to('../LICENSE')
        self.link(a, extra)
        found = notices.license_texts(self.packages[extra])
        self.assertEqual(found, [('LICENSE', 'real')])
        self.assertNotIn('outside secret', self.render())

    def test_license_file_field_outside_the_crate_is_rejected(self):
        app, a, b = self.graph()
        (self.registry / 'escape.txt').write_text('outside the crate')
        escaping = self.add('escaping', license=None, license_file='../escape.txt')
        self.link(a, escaping)
        with self.assertRaises(ValueError):
            notices.license_texts(self.packages[escaping])
        # A symlinked license_file that leaves the crate is rejected the same way.
        linked = self.add('linkfield', license=None, license_file='TERMS')
        (self.registry / 'linkfield-1.0.0/TERMS').symlink_to(self.registry / 'escape.txt')
        with self.assertRaises(ValueError):
            notices.license_texts(self.packages[linked])

    def test_license_file_field_is_included_even_outside_the_name_patterns(self):
        app, a, b = self.graph()
        custom = self.add('custom', license=None, license_file='legal/TERMS',
                          files={'legal/TERMS': 'custom terms\n', 'LICENSE': 'custom terms\n'})
        self.link(a, custom)
        found = dict(notices.license_texts(self.packages[custom]))
        self.assertEqual(found, {'LICENSE': 'custom terms', 'legal/TERMS': 'custom terms'})
        text = self.render()
        self.assertIn('declared through license-file', text)

    def test_identical_texts_are_printed_once_and_referenced_by_crate(self):
        self.graph()
        text = self.render()
        self.assertEqual(text.count('MIT fixture text\nSecond line'), 1)
        self.assertEqual(text.count('Apache fixture text'), 1)
        self.assertEqual(text.count('beta notice'), 1)
        self.assertIn('alpha 1.2.0', text)
        self.assertIn('beta 0.3.0', text)
        # Both crates point at the same shared text number.
        self.assertRegex(text, r'alpha 1\.2\.0[\s\S]*?LICENSE-MIT \[(\d+)\][\s\S]*?beta 0\.3\.0[\s\S]*?license-mit \[\1\]')

    def test_crates_without_license_files_are_listed_explicitly_without_invented_text(self):
        app, a, b = self.graph()
        bare = self.add('bare', '2.0.0', 'MIT OR Unlicense', {'README.md': 'nothing legal'})
        self.link(a, bare)
        text = self.render()
        section = text.split('Crates without a license file in the published package')[1].split('\n\n\n')[0]
        self.assertIn('bare 2.0.0', section)
        self.assertIn('MIT OR Unlicense', section)
        self.assertIn('published crate ships no license file', section)
        self.assertNotIn('alpha', section)

    def test_vendored_path_dependency_uses_relative_location_and_is_listed(self):
        app, a, b = self.graph()
        vendored = self.add('glibish', '0.18.5', 'MIT', {'LICENSE': 'glib mit', 'COPYRIGHT': 'glib copyright'},
                            directory=self.workspace / 'vendor/glibish-0.18.5', source=None)
        # Path packages that are not members have no registry source.
        self.packages[vendored]['source'] = None
        self.packages[vendored]['id'] = vendored
        self.link(a, vendored)
        text = self.render()
        self.assertIn('glibish 0.18.5', text)
        self.assertIn('vendored in this repository: desktop/vendor/glibish-0.18.5', text)

    def test_output_is_deterministic_sorted_and_free_of_absolute_paths_and_timestamps(self):
        app, a, b = self.graph()
        extra = self.add('aaa', files={'LICENSE': 'aaa text'}, repository='https://example.test/aaa')
        self.link(app, extra)
        first = self.render()
        data = self.metadata()
        random.Random(7).shuffle(data['packages'])
        random.Random(8).shuffle(data['resolve']['nodes'])
        for node in data['resolve']['nodes']:
            random.Random(9).shuffle(node['deps'])
        self.assertEqual(notices.render(data), first)
        self.assertNotIn(str(self.root), first)
        self.assertNotIn(self.temp.name, first)
        self.assertNotIn('/Users/', first)
        self.assertLess(first.index('aaa 1.0.0'), first.index('alpha 1.2.0'))
        self.assertLess(first.index('alpha 1.2.0'), first.index('beta 0.3.0'))
        first.encode('utf-8')
        self.assertNotIn('\r', first)
        self.assertTrue(first.endswith('\n'))
        self.assertFalse(first.endswith('\n\n'))

    def test_crlf_and_trailing_blank_lines_are_normalized_before_deduplication(self):
        app, a, b = self.graph()
        crlf = self.add('crlf', files={'LICENSE-MIT': MIT.replace('\n', '\r\n') + '\r\n\r\n'})
        self.link(a, crlf)
        self.assertEqual(self.render().count('MIT fixture text'), 1)

    def test_non_utf8_license_text_is_rejected_not_silently_mangled(self):
        app, a, b = self.graph()
        bad = self.add('bad')
        (self.registry / 'bad-1.0.0/LICENSE').write_bytes(b'\xff\xfe invalid')
        self.link(a, bad)
        with self.assertRaises(ValueError):
            self.render()

    def write_repo_metadata(self):
        self.graph()
        repo = self.root / 'repo'
        repo.mkdir()
        return repo

    def test_generate_writes_only_the_notices_file_and_is_idempotent(self):
        repo = self.write_repo_metadata()
        (repo / 'LICENSE').write_text('untouched')
        with patch.object(notices, 'load_metadata', return_value=self.metadata()):
            self.assertEqual(notices.main(['generate'], repo=repo), 0)
            self.assertEqual(sorted(path.name for path in repo.iterdir()), ['LICENSE', 'THIRD-PARTY-NOTICES'])
            first = (repo / 'THIRD-PARTY-NOTICES').read_bytes()
            self.assertEqual(notices.main(['generate'], repo=repo), 0)
            self.assertEqual((repo / 'THIRD-PARTY-NOTICES').read_bytes(), first)
            self.assertEqual((repo / 'LICENSE').read_text(), 'untouched')

    def test_generate_refuses_a_symlink_or_directory_target(self):
        repo = self.write_repo_metadata()
        target = repo / 'THIRD-PARTY-NOTICES'
        victim = repo / 'victim'
        victim.write_text('keep')
        target.symlink_to(victim)
        with patch.object(notices, 'load_metadata', return_value=self.metadata()):
            with self.assertRaises(SystemExit):
                notices.main(['generate'], repo=repo)
        self.assertEqual(victim.read_text(), 'keep')
        target.unlink()
        target.mkdir()
        with patch.object(notices, 'load_metadata', return_value=self.metadata()):
            with self.assertRaises(SystemExit):
                notices.main(['generate'], repo=repo)

    def test_check_accepts_current_file_and_rejects_missing_or_stale_ones_with_a_clear_message(self):
        repo = self.write_repo_metadata()
        with patch.object(notices, 'load_metadata', return_value=self.metadata()):
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                self.assertNotEqual(notices.main(['--check'], repo=repo), 0)
            self.assertIn('THIRD-PARTY-NOTICES', stderr.getvalue())
            self.assertIn('generate', stderr.getvalue())
            self.assertEqual(notices.main(['generate'], repo=repo), 0)
            self.assertEqual(notices.main(['--check'], repo=repo), 0)
            target = repo / 'THIRD-PARTY-NOTICES'
            target.write_text(target.read_text() + 'stale\n')
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                self.assertNotEqual(notices.main(['--check'], repo=repo), 0)
            self.assertIn('differs', stderr.getvalue())
            self.assertIn('generate', stderr.getvalue())
            # --check never rewrites the committed file.
            self.assertTrue(target.read_text().endswith('stale\n'))

    def test_usage_errors_are_rejected(self):
        repo = self.write_repo_metadata()
        for argv in [[], ['--bogus'], ['generate', '--check']]:
            with self.subTest(argv=argv):
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    notices.main(argv, repo=repo)

    def test_cargo_metadata_command_is_locked_offline_and_platform_filtered(self):
        with patch.object(notices.subprocess, 'run') as run:
            run.return_value.stdout = json.dumps({'ok': True})
            self.assertEqual(notices.load_metadata(Path('/repo')), {'ok': True})
        args = run.call_args.args[0]
        for expected in ['--format-version', '1', '--locked', '--offline', '--filter-platform',
                         'aarch64-apple-darwin', '--manifest-path']:
            self.assertIn(expected, args)
        self.assertEqual(args[args.index('--manifest-path') + 1], '/repo/desktop/Cargo.toml')


if __name__ == '__main__':
    unittest.main()
