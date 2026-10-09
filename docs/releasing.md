# Release verified macOS artifacts

The development version is **0.1.0-beta.3, unreleased**, bundle build **3**.
The published manual download remains beta.2; preserve beta.1/beta.2 tags and
installers. The first updater-capable release requires manual installation.
Commits, pushes, tags, secrets/settings changes, workflow execution, Releases
and feed publication need explicit authorization in the delivery task.

The user reported that the beta.2 DMG works, without an individual GUI case
record. That evidence applies to installer SHA256
`1a811ad7459ae9d69f923693052228812e2a11b2684c455d558620a1956e203b`,
not to beta.3. Packaging, runtime, CI and download checks remain separate.

## 1. Review and check exact sources

```sh
python3 scripts/test-version.py
python3 scripts/check-version.py --tag "v$(cat VERSION)"
bash desktop/scripts/test.sh
python3 desktop/native/Tests/Branding/check.py
bash desktop/scripts/build.sh
git status --short
```

`VERSION` is the full distribution SemVer; Cargo/Tauri/plist use its numeric
base and the plist build number identifies the bundle. The local build keeps
the custom native dylib/resources bundle, debug profile and ad-hoc Apple
signature. Independent review must cover the exact source and artifacts.
Before release, commit/review intended changes and require the clean exact
`v<VERSION>` tag; never overwrite a published installer or move a tag.

`build.sh` snapshots tracked/nonignored sources before compilation, invalidates
the old receipt, recreates the known app output, verifies deep/strict signatures
and compares the source snapshot afterward. It writes
`desktop/.build/build-source.json` only on success, with VERSION, source hashes,
HEAD/dirty status and bundle hashes/modes/internal symlinks. This is an observed
build receipt, not a substitute for independent review or reproducible-build
proof. A dirty build retains its exact source digest; its compiled commit is
null. Further source or bundle changes invalidate updater packaging.

## 2. Package manual and signed updater candidates

```sh
bash desktop/scripts/package-dmg.sh
python3 desktop/scripts/package-updater.py package \
  --key-file "$HOME/Library/Application Support/UploadIMG/signing/updater.key"
VERSION="$(cat VERSION)"
(cd "desktop/.build/distribution/$VERSION" && shasum -a 256 -c SHA256SUMS)
(cd "desktop/.build/distribution/$VERSION-updater" && shasum -a 256 -c UPDATER-SHA256SUMS)
```

The DMG script packages the reviewed existing bundle without rebuilding or
re-signing it, verifies UDZO, mounts read-only, compares content/instructions,
checks the Applications link and detaches the exact device. Its output is
`desktop/.build/distribution/<VERSION>/UploadIMG-<VERSION>-macos-arm64.dmg`.
The DMG provenance records packaging-time source/bundle/artifact identities;
`compiled_source_commit` remains null because packaging-time HEAD alone does
not establish compilation origin. Retain the separate matching build receipt.

The updater helper requires the current build receipt, checks metadata/icon,
arm64/rpaths/dependencies and Apple signatures, and produces
`UploadIMG-<VERSION>-macos-arm64.app.tar.gz`, `.sig`, a candidate channel JSON,
`updater-provenance.json` and `UPDATER-SHA256SUMS` in `<VERSION>-updater/`. It compares extracted
bytes, modes and internal symlinks; unsafe links/wrong layout are rejected.
The official isolated `tauri-cli 2.12.1` signs with a private key **file path**
and binds the full version. Signature structure/key ID checks are not crypto
verification. Run the public-data verifier as a separate gate:

```sh
VERSION="$(cat VERSION)"
OUTPUT="desktop/.build/distribution/$VERSION-updater"
CARGO_TARGET_DIR="$PWD/desktop/.build/cargo-target" \
DYLD_LIBRARY_PATH="$PWD/desktop/.build/native" \
cargo run --offline --locked --jobs 2 --manifest-path desktop/Cargo.toml \
  -p uploadimg-desktop --bin verify-update -- \
  "$OUTPUT/UploadIMG-$VERSION-macos-arm64.app.tar.gz" \
  "$OUTPUT/UploadIMG-$VERSION-macos-arm64.app.tar.gz.sig" \
  desktop/updater-public-key.txt "$VERSION"
```

Both packaging paths refuse existing outputs. Retries require new directories
inside `desktop/.build/distribution`; preserve failed staging/attachment
receipts for inspection. Do not edit provenance to imply a clean compiled
commit or checks that were not run. Archive signatures are separate from
Apple signing; there is no Developer ID or notarization.

## 3. Verify runtime and build automation

Local updater evidence from 2026-10-09 covers a real signed beta.3→beta.4
replacement of a disposable macOS bundle using official updater 2.13.2,
Tauri `MockRuntime` and HTTPS localhost with an explicit test CA. Both bundle
executables reported their versions; permissions/native library/public key/plist
were checked. Bad signatures and signed-version mismatch left the old bundle
unchanged. The separate crypto helper accepted a valid archive and rejected
altered bytes, wrong key and version mismatch. Beta.4 is a local fixture only.
See [the precise evidence boundary](automatic-updates.md#verified-local-boundary).

Observe the exact version's startup, tray, preview, explicit sharing and opt-in
detection using synthetic data. GUI/update confirmation, actual restart,
Gatekeeper and recovery after a real filesystem installation failure remain
NOT_RUN; verify them on a disposable installation. Keep cancellation/busy-state
regressions and the headless replacement result distinct from graphical runtime.
Do not use the owner's installed app as the experiment. Network/upload, CI and
public download verification also remain separate evidence. Rebuild and package
the final reviewed source after documentation changes so receipts match it.

The [manual macOS arm64 workflow](../.github/workflows/build-macos.yml) fetches
locked dependencies, runs checks and builds the custom bundle using Rust
1.92.0/Node 24.21.0. Its official Actions are pinned to full SHAs verified from
[checkout v6.1.0](https://github.com/actions/checkout/releases/tag/v6.1.0),
[setup-node v7.1.0](https://github.com/actions/setup-node/releases/tag/v7.1.0) and
[upload-artifact v7.0.0](https://github.com/actions/upload-artifact/releases/tag/v7.0.0).
The [documented macos-26 runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
is arm64; actual toolchain/runtime identity is recorded by workflow logs.
It requires `UPLOADIMG_UPDATER_PRIVATE_KEY` and optionally
`UPLOADIMG_UPDATER_KEY_PASSWORD`. The owner must provision them separately;
no secrets or settings are changed by the local implementation task.
Missing key fails signing clearly. Dispatch from a tag verifies exact VERSION.
The workflow verifies signed artifacts and uploads candidates only, with
read-only contents permission. It has no release/feed publication job.
Configuration/schema checks do not establish executed CI or app runtime.

## 4. Publish and verify downloads after authorization

1. Push the independently reviewed source commit and exact `v<VERSION>` tag.
2. Create a draft prerelease in `SergioGMR/uploadimg-desktop`.
3. Upload new DMG/updater archives, matching signature, checksums and provenance
   without asset clobber. Keep the distinct DMG `provenance.json`/`SHA256SUMS`
   and updater `updater-provenance.json`/`UPDATER-SHA256SUMS` names, with each
   artifact's associated source receipt.
4. Include full version, source/artifact identity, debug/ad-hoc status, absent
   notarization and actual device/runtime/CI checks. Review then publish only
   within the explicitly authorized delivery task.
5. Download every published asset into a fresh directory and compare exact
   bytes/checksums against the accepted originals.
6. Only after independent update/recovery acceptance and download verification,
   publish the generated candidate as `updates/beta.json` or `updates/stable.json`
   in a separately authorized commit/push. No placeholder feeds or future-asset
   pointers belong in main before that gate.

Download integrity does not prove Gatekeeper or installed runtime. A correction
requires a new version/tag, while preserving all previous published assets.
See [automatic updates](automatic-updates.md) for key custody, feed contract,
transport limits and staged rollout.
