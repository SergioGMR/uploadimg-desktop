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

Public source visibility does not grant an open-source license. No license has been
selected for this repository; agree on contribution and reuse terms with the owner.
