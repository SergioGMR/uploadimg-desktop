# Contributing

Start with an issue describing the observable problem or requested outcome.
Before beginning work, maintainers review open issues for duplicates and related work,
record the chosen scope and connect the eventual pull request. Issue monitoring is manual.

## A reviewable change

1. Agree on the behavior, affected platforms and acceptance criteria in the issue.
2. Keep implementation, regression tests and relevant documentation together.
3. Run the checks below and record actual results, including skipped device checks.
4. Link the issue in the pull request. Close it only after its acceptance criteria are verified.

Version checks require **Python 3.11 or newer** for the standard-library `tomllib` module.

```sh
python3 scripts/test-version.py
python3 scripts/check-version.py
bash desktop/scripts/test.sh
```

## Versions and releases

Use [SemVer](https://semver.org/): `MAJOR.MINOR.PATCH`, optionally with a prerelease
such as `-beta.2`. Before 1.0, describe breaking changes explicitly in the changelog.
Update `VERSION`, the matching `CHANGELOG.md` entry and the numeric base in both
Cargo package manifests, Tauri configuration and `Info.plist` together.
Tags must equal `v<VERSION>`. Never reuse a tag or replace a published installer.
See [the release procedure](docs/releasing.md) for packaging and publication gates.

## License and attribution

Original project material is licensed under GNU GPL version 3 only
([GPL-3.0-only](https://spdx.org/licenses/GPL-3.0-only.html)). See [LICENSE](LICENSE)
for the full terms and [COPYRIGHT](COPYRIGHT) for the project notice.
Submit original contributions under the same license and identify any third-party
material with its origin, copyright notices and compatible license. No additional
contributor agreement is required by this guide.

Preserve copyright and license notices. Modified versions must carry prominent
change notices and relevant dates; distributing a covered derivative requires
GPL-3.0-only terms and access to its Corresponding Source under the GPL.
Third-party components keep their own notices and licenses, listed in
[THIRD-PARTY-NOTICES](THIRD-PARTY-NOTICES) (generated; after a dependency change run
`python3 desktop/scripts/third-party-notices.py generate` and commit the result),
including the vendored GLib MIT [LICENSE](desktop/vendor/glib-0.18.5/LICENSE) and
[COPYRIGHT](desktop/vendor/glib-0.18.5/COPYRIGHT).
