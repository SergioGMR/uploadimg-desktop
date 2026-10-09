# Release a verified installer

The current prerelease is `0.1.0-beta.2`, distributed as **UploadIMG.app** in
`UploadIMG-0.1.0-beta.2-macos-arm64.dmg`. The published `0.1.0-beta.1` remains
the historical simulation version; preserve its tag and installer. Commits,
pushes, tags and release publication need explicit authorization.

The user tested the local beta.2 DMG and reported that it works; individual
GUI cases were not recorded. That confirmation applies to installer SHA256
`1a811ad7459ae9d69f923693052228812e2a11b2684c455d558620a1956e203b`.
Packaging checks, remote publication and download verification are separate evidence.

## 1. Review and check the source

```sh
python3 scripts/test-version.py
python3 scripts/check-version.py --tag "v$(cat VERSION)"
bash desktop/scripts/test.sh
python3 desktop/native/Tests/Branding/check.py
python3 desktop/scripts/tests/test-package-manifest.py
bash desktop/scripts/build.sh
git status --short
```

`VERSION` is the distribution SemVer. Cargo, Tauri and the plist short version
use its numeric base; the plist build number identifies the new bundle.
Review the exact source and built artifact independently. Observe startup,
menu behavior, preview, explicit sharing and opt-in detection with synthetic
data; report device and upload checks separately from automated checks.
Before publishing, commit and review intended changes and require a clean
worktree for the exact `v<VERSION>` tag.

## 2. Package the reviewed local bundle

After its build and checks finish:

```sh
bash desktop/scripts/package-dmg.sh
cd "desktop/.build/distribution/$(cat VERSION)" # run from repository root
shasum -a 256 -c SHA256SUMS
```

The script reads `VERSION`, packages the existing debug `UploadIMG.app`, and
never rebuilds or re-signs it. The default output is
`desktop/.build/distribution/<VERSION>/UploadIMG-<VERSION>-macos-arm64.dmg`.
It refuses existing outputs; a retry accepts a new subdirectory inside
`desktop/.build/distribution`. Preserve failed staging/attachment receipts
until inspected. Never overwrite a published artifact.

Checks cover current source/bundle metadata and icon, arm64 binaries,
portable dependencies/rpaths and ad-hoc signatures. The script verifies the
UDZO image, mounts it read-only, compares bundle hashes and installation
notes, checks the Applications symlink, and detaches the exact device.
`SHA256SUMS` covers the DMG and `provenance.json`.

Provenance records packaging-time Git HEAD, dirty status, a digest and hashes
of tracked/nonignored source files, plus the complete bundle file manifest.
`compiled_source_commit` remains null: packaging-time sources do not prove
which sources compiled a prebuilt bundle. Keep build/review/runtime receipts
for the same source and artifact boundary; do not edit provenance to imply
a clean-commit build. DMG bytes can vary between runs.

## 3. Draft, upload and verify downloads

1. With explicit authorization, push the reviewed commit and exact version tag.
2. Create a draft prerelease for `v<VERSION>` in `SergioGMR/uploadimg-desktop`.
3. Upload the new DMG, `SHA256SUMS` and `provenance.json` without asset clobber.
4. Include the changelog, source/artifact identities, debug profile, actual
   macOS/device/upload checks, ad-hoc signing and absent notarization.
5. Verify draft metadata/assets, then publish only within the authorized task.
6. Download all published assets to a new directory, compare their bytes to
   the local originals, and verify `SHA256SUMS`.

Download integrity does not establish installed runtime or Gatekeeper behavior.
An incorrect published installer requires a new version and tag.
