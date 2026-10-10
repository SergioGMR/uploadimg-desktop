# Automatic updates for macOS arm64

**Published beta: 0.1.0-beta.3, released on 2026-10-10.** The exact-source
macOS build workflow succeeded, the release was independently reviewed and all
nine published assets were downloaded anonymously and verified. Install beta.3
manually when upgrading from beta.1/beta.2, which do not contain an updater.
The beta channel manifest points to beta.3; a same-version in-app check is
expected to report up to date, but that live check remains separate evidence.

A historical headless macOS test performed real replacement of a disposable
beta.3 bundle with a signed beta.4 fixture. That historical fixture is separate
from the prepared **0.1.0-beta.4** candidate; neither establishes its publication.
GUI update confirmation, automatic restart, Gatekeeper and recovery after an actual
filesystem installation failure remain **NOT_RUN**. This authorized beta.3
rollout is a limited exception, not acceptance of the full update/recovery/
restart matrix or a change to the general release gate.

Exact-source CI, independent source/artifact review and public-download
verification remain pending for the beta.4 candidate. The published beta channel
still targets beta.3; no beta.4 feed activation or actual GUI updater result is
established by this preparation. See [the release procedure](releasing.md) for
the candidate's delivery gate.

The beta.4 candidate retains the stable bundle identifier and updater public key.
It restores only the macOS boolean `clipboardIntent` from the existing
`UserDefaults` domain after normal restart or app replacement. Fresh or invalid
preferences default OFF; an earlier stored choice is preserved. Pause is
temporary and restoration takes a fresh clipboard baseline without admitting
old content. Images, previews, links and upload recovery remain in memory;
asynchronous preference writes do not promise abrupt-termination durability.

## Application and safety boundary

The local integration uses pinned Tauri `=2.12.2`, tauri-build `=2.7.1`,
updater `=2.13.2` and semver `=1.0.28`. Updater 2.12.0 was rejected because
its macOS failed-install path could delete the previous app. The
[upstream recovery fix](https://github.com/tauri-apps/plugins-workspace/commit/2a9c29e004325db674fd904089749c38f4aac824)
adds atomic replacement where supported and restoration/retained backup paths.
That source fix does not prove crash recovery on every filesystem.

Rust embeds the full `VERSION`; Cargo/Tauri/plist retain the numeric base.
Beta feeds can promote to stable; stable installations reject prereleases.
Only strictly newer SemVer precedence is eligible: beta.10 follows beta.9;
equal, older and build-metadata-only changes are rejected. Checks happen
without opening/focusing the window; installation and restart require an
explicit user action for the offered version. Missing/invalid configuration
and feed/download failures must remain visible, never appear as up to date.

Installation must be blocked while pending images, active uploads or
unresolved recovery capabilities remain. Restart discards in-memory result
history; users can cancel before confirming. A lifecycle guard prevents new
admissions or late queued work while the updater owns replacement/restart.
Failures must not restart automatically. `PreviousAppNotRestored` needs a
visible retained-backup path and manual recovery instructions. Real replacement
has headless disposable-bundle evidence below; graphical installation/restart,
rollback after an actual filesystem failure and Gatekeeper still require checks.

Only the Rust plugin performs updater networking; no JavaScript updater
permissions or npm packages are added. Image sharing remains explicit.

## Verified local boundary

The 2026-10-09 integration harness used the official updater 2.13.2 with
Tauri `MockRuntime` and an HTTPS localhost server trusted through an explicit
test CA. The WebView/tray runtime was mocked; archive download, signature
verification and filesystem replacement were real. It replaced only a
disposable beta.3 bundle with beta.4, ran each bundle's executable `--version`
to confirm both versions, and checked retained executable permissions, native
library, public key and plist. Bad signatures and a signed-version mismatch
were rejected without changing the old bundle.

The separate official-minisign helper accepted the real signed archive and
rejected altered bytes, a wrong public key and a mismatched signed version.
These checks are distinct from the deliberately invalid format fixtures.
Local receipts are `desktop/.build/updater-runtime/integration-20261009T185500Z/`
(`runtime.log` and `signature-checks.json`). The runtime test passed once;
the 137 automated regression checks are a separate result.

This harness did not exercise GUI confirmation, application restart,
Gatekeeper, a downloaded public Release, or recovery after an actual filesystem
replacement failure. It did not modify the owner's installed application or
publish beta.4. Those historical fixture receipts are separate from the
reviewed beta.3 source/artifact and public-download receipts below.

## Beta.3 release evidence

The [beta.3 Release](https://github.com/SergioGMR/uploadimg-desktop/releases/tag/v0.1.0-beta.3)
was published on 2026-10-10 from source
`085264549565eb1c40261cfd959fcaa3f1c644c4`. The
[build run](https://github.com/SergioGMR/uploadimg-desktop/actions/runs/38066375725)
succeeded on macOS 26.6.2 arm64 with 137 checks: 68 Rust, 17 Swift, 29 Node,
15 packaging and 8 version checks. Two opt-in integration tests were skipped;
CI did not repeat the headless replacement experiment or prove GUI runtime.

Independent review accepted the exact source, nine artifacts and release notes.
All nine assets were then downloaded by anonymous HTTPS and matched their
accepted originals. Both checksum manifests, clean build/source receipts,
matching DMG/updater bundle contents and modes, version-bound updater signature,
DMG integrity/read-only mount, Apple signatures, packaged notes and the bundled
executable's beta.3 `--version` output were checked on those downloads.
Local receipts are under `desktop/.build/release-beta3/`, including
`public-download-verification.json` and `public-verification/verification.json`.

This exact beta was authorized for limited release and beta-feed activation
after independent artifact review and download verification, with the complete
update/recovery/restart matrix explicitly pending. The usual gate still requires
that matrix; this exception does not mark it passed or apply to later releases.
The owner's installed application was not used as an installation experiment.

## Network limits

Feed and archive requests use HTTPS with allowed origins and a 15-second
request timeout. The official plugin's feed check deserializes its response
with `res.json().await`; this integration does **not** impose a hard response
byte cap on that path. The publication helper limits generated feeds to
64 KiB, which controls our generated files, not arbitrary responses in transit.

Archive downloads have a 256 MiB application limit with cancellation and a
final size check. A chunk can already have been received before cancellation,
so this is not a hard transport allocation bound. There is no insecure
transport/signature bypass. These limitations remain part of the beta's
review and acceptance boundary.

## Signed artifacts and feeds

The [official Tauri updater format](https://v2.tauri.app/plugin/updater/) is a
macOS `.app.tar.gz` and matching `.sig`. The static manifest contains the full
version and only `platforms.darwin-aarch64`, with an HTTPS release asset URL
and signature **contents**, never a signature URL.

```sh
bash desktop/scripts/build.sh
python3 desktop/scripts/package-updater.py package \
  --key-file "$HOME/Library/Application Support/UploadIMG/signing/updater.key"
```

The helper defaults to the isolated signer
`desktop/.build/tools/tauri-cli-2.12.1/bin/cargo-tauri`; `--signer` accepts the
owner's verified path. The [official signer command](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.1/crates/tauri-cli/src/signer/sign.rs)
uses `--private-key-path FILE --app-version VERSION`. Python never reads the
private key and signer output is suppressed. If the key is password protected,
provide `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` through the owner's secure session;
never put the password on argv or in logs. Missing/invalid keys and failed
signing stop signed packaging. The private key must remain outside Git with
owner-only permissions; retain an owner-controlled backup. Only
`desktop/updater-public-key.txt` belongs in the repository.

A new output directory `desktop/.build/distribution/<VERSION>-updater/` contains:

- `UploadIMG-<VERSION>-macos-arm64.app.tar.gz` and `.app.tar.gz.sig`.
- `beta.json` or `stable.json`, generated from the actual signature.
- `updater-provenance.json` and `UPDATER-SHA256SUMS` (distinct from DMG assets).

The helper refuses an existing directory; `--output` permits a fresh retry
subdirectory under `desktop/.build/distribution`. It requires a current
`build-source.json` receipt, preserves bundle bytes/permissions/internal
symlinks, rejects escaped/dangling links, compares extracted contents and
checks deep/strict Apple signatures. It validates signature encoding, key ID
and bound version; those are format checks, **not cryptographic verification**.
The separate Rust `verify-update` helper verifies the signature with official
minisign code before release acceptance; unit-test format fixtures deliberately
contain invalid zero signatures and cannot satisfy that gate.

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
(cd "$OUTPUT" && shasum -a 256 -c UPDATER-SHA256SUMS)
```

`updates/beta.json` is a byte-for-byte copy of the verified manifest downloaded
from the beta.3 Release. Its production endpoint is
`https://raw.githubusercontent.com/SergioGMR/uploadimg-desktop/main/updates/beta.json`;
it contains only `platforms.darwin-aarch64` and the exact published beta.3 archive
URL/signature. Feed commit/push, raw-endpoint verification and the live in-app
query require separate delivery checks; copying the manifest does not establish
them. `updates/stable.json` remains absent until a stable release meets its gate.
Beta does not use `releases/latest`, which excludes prereleases. Future candidate
manifests still need independent acceptance and verified published downloads
before channel activation; generated URLs alone do not prove publication.

## Build automation and release gates

The [macOS workflow](../.github/workflows/build-macos.yml) runs only through
`workflow_dispatch`, uses GitHub's documented arm64 `macos-26` runner, Rust
1.92.0 and Node 24.21.0, fetches locked dependencies and builds our custom
bundle. It then produces the manual DMG and updater candidates, runs the
signature verifier and uploads workflow artifacts for independent review.
Actions are pinned to full official commit SHAs. Source metadata/tag checks
run before compilation; a dispatch from a tag must match `v<VERSION>`.

Required secret: `UPLOADIMG_UPDATER_PRIVATE_KEY`; optional password secret:
`UPLOADIMG_UPDATER_KEY_PASSWORD`. The signing step creates an owner-only file
in `RUNNER_TEMP` and removes it in `finally`. With no signing secret, the step
fails clearly. The owner separately provisioned the signing secret; the beta.3
workflow's signing and verification steps succeeded. That execution establishes
operational signing without disclosing the secret. Repository contents
permission is read-only; no Release/feed publication job exists.

The app remains debug/ad-hoc signed with no Developer ID or notarization.
Updater signing is a separate trust mechanism and proves neither Gatekeeper
acceptance nor runtime. First independently review the exact source/artifact;
then observe good/bad/wrong-key signatures, old-to-new installation, permissions,
rollback/recovery and restart on a disposable macOS installation. Record checks
actually run separately from pure unit/format tests, CI and download integrity.
Publication, feed commits/pushes and remote settings still require explicit
authorization. Preserve every published tag/asset; corrections need a new version.

See [the release procedure](releasing.md) for manual packaging and final delivery.
