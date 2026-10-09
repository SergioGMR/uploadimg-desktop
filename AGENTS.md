# Uploadimg Desktop maintenance

## Scope and evidence

This repository contains a local desktop beta. Read README.md and CONTRIBUTING.md before work. Preserve the distinction between simulated sharing and real uploads, and between tested macOS behavior and planned Windows/Linux support. Never describe a check as passed unless it was executed.

## Versions and releases

- VERSION is the authoritative distribution version; follow SemVer 2.0.0 and docs/releasing.md.
- Keep package metadata and CHANGELOG.md consistent. Run `python3 scripts/test-version.py` and `python3 scripts/check-version.py` after changes to versioning.
- Release tags are exactly `v<VERSION>`. Never move a published tag or overwrite a published installer; corrections require a new version.
- A release needs independent review, checks for its exact source/artifact and download verification. Metadata CI alone does not prove application runtime.
- Commits, pushes, releases and repository settings require authorization in the current task. These instructions do not grant remote mutation permission.

## Issues

- At the start of relevant implementation work, inspect related open and closed issues in `SergioGMR/uploadimg-desktop` using the authorized GitHub session; if unavailable, report the limitation.
- Follow the issue forms and CONTRIBUTING.md. Link the relevant issue in change descriptions and verify its acceptance criteria before reporting completion.
- Do not create duplicates, invent reports, or close issues automatically without the authorized task covering that action.
- Issue review happens during active work. Do not claim continuous monitoring or scheduled triage unless an actual automation is configured and verified.

## Local-only files

Respect .gitignore. Build caches, installers, session receipts, odd/ coordination and the separate native-probe/ experiment stay local. Publish installers through GitHub Releases, with checksums and honest provenance.
