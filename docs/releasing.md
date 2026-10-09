# Release a verified installer

Publish only after source, version metadata and the exact installer have been
reviewed. The current packager preserves the existing tested debug bundle for
`0.1.0-beta.1`; it neither builds nor re-signs it. Its local snapshot receipts are
excluded from Git. A fresh clone can build the app but cannot reproduce this beta's
packaging checks without those original receipts and bundle.

## 1. Check the source and version

```sh
python3 scripts/test-version.py
python3 scripts/check-version.py --tag "v$(cat VERSION)"
bash desktop/scripts/test.sh
git status --short
```

Use `VERSION` as the authoritative distribution SemVer. Keep both Cargo package
versions, Tauri `version` and plist `CFBundleShortVersionString` equal to its numeric
base; keep a matching changelog heading. Commit all intended source/docs/checks,
review the commit and require a clean worktree before tagging `v<VERSION>`.
Record the full source commit, tag and check results in the release notes.

## 2. Package the exact tested bundle

```sh
bash desktop/scripts/package-dmg.sh
cd desktop/.build/distribution
shasum -a 256 -c SHA256SUMS
```

The script verifies the recorded beta bundle, portable dependencies, ad-hoc
signatures and arm64 architecture; mounts the DMG read-only, compares bundle bytes,
then detaches it. It refuses existing outputs. A retry uses a new subdirectory
inside `desktop/.build/distribution`; never overwrite an existing artifact.
The bundle evidence predates repository initialization: `provenance.json` records
its source snapshot and file hashes, with `packaged_source_commit` initially null.
After the publication/packaging source commit is reviewed, record its full SHA in
`packaged_source_commit` and recompute the `provenance.json` entry in `SHA256SUMS`
before uploading. Do not alter the DMG. This SHA identifies the published packaging
sources; it does not claim that the pre-Git bundle was compiled from that commit.
Release notes must distinguish the original snapshot from the later public commit.

For subsequent releases, prepare the packaging procedure for the new source
boundary, build the intended release-profile artifact, and validate that exact
artifact at runtime before publication. Check install/open, menu behavior, image
preview, explicit sharing and opt-in detection with synthetic test data. Record
results separately from automated checks. Never claim untested platforms.

## 3. Draft, upload, then publish

1. Create and push the reviewed commit and exact tag only with explicit authorization.
2. Create a **draft prerelease** for `v0.1.0-beta.1` in `SergioGMR/uploadimg-desktop`.
3. Upload `Uploadimg-Desktop-0.1.0-beta.1-macos-arm64.dmg`, `SHA256SUMS` and
   `provenance.json` from the verified output directory. Never use asset clobber.
4. Include the changelog, source commit, debug profile, macOS 26.6.2 arm64 evidence,
   simulation limits, ad-hoc signing and absent Apple notarization in the notes.
5. Verify draft metadata and the uploaded asset set before publishing the prerelease.
6. Download the published assets into a new temporary directory; compare each
   downloaded file byte-for-byte with its local original and verify `SHA256SUMS`.

Download integrity does not establish installed runtime or Gatekeeper behavior.
Record those checks explicitly. If an installer is wrong, withdraw the release and
publish a corrected version with a new tag; never silently replace published bytes.
