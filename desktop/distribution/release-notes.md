# UploadIMG 0.1.0-beta.5

This published macOS arm64 beta stops offering files and folders copied in Finder
for sharing, and adopts the GPL-3.0-only license with the legal notices bundled
in the app. Beta.3 was the first updater-capable version; install manually from
beta.1/beta.2, which do not contain an updater. From beta.3 or beta.4, use
**Buscar actualizaciones** and confirm the offered installation.
Published on 2026-11-03 UTC from source `aa0f0fbba8faa7c71ff32686c146215c5c0c82df`.
These repository notes were updated afterward; published packages and their
tagged-source notes remain unchanged.

## What changed

- **Finder copies are no longer offered.** Copying a file or folder in Finder
  also places its icon on the clipboard as an image. Clipboard detection and
  **Pegar imagen** now treat that combination as unsupported, so a Finder file
  or folder copy is not offered for sharing. Image files copied in Finder are not
  shared through their icon either. Images copied from apps that also carry a
  file reference remain supported.
- **License.** Original project material is licensed under GPL-3.0-only. The app
  bundles `LICENSE`, `COPYRIGHT` and `THIRD-PARTY-NOTICES` (licenses of the Rust
  dependencies) unchanged in `UploadIMG.app/Contents/Resources`. The vendored
  GLib keeps its MIT notices. The notices do not claim an exhaustive license audit.
  Known limitation: `THIRD-PARTY-NOTICES` lists 13 crates (the objc2 family,
  block2, alloc-stdlib, selectors and dispatch2) without license text because
  the published crates ship no license file; a later version is planned to address it.

## License and Corresponding Source

The exact Corresponding Source of this release is the source archive of the tag,
`UploadIMG-0.1.0-beta.5-source.tar.gz`, attached to this Release as an asset next
to the binaries. It is produced by `git archive` of the tag `v0.1.0-beta.5`.
Dependency sources are not copied into the archive: obtain them from crates.io as
pinned by `desktop/Cargo.lock` (run `cargo fetch --locked` in `desktop/`).
The build scripts snapshot Git-tracked sources, so build an extracted archive inside
a new repository (`git init`, `git add -A`, `git commit`).
The attached archive was downloaded anonymously after publication and matched
the archive regenerated from the tag byte for byte (sha256
`76d492a981183e1822f57b619eed570ccbeb46bb989fc0d42c46b43e80296d1c`).

## Package

The DMG contains **UploadIMG.app**, an Applications shortcut and installation
notes. It packages the reviewed custom debug bundle without compiling,
re-signing or launching it. Application short version: **0.1.0**; build **5**.
The bundle uses an Apple ad-hoc signature with no Developer ID/notarization.
The updater archive uses a separate dedicated Tauri/minisign signature.

Clipboard detection starts OFF with a fresh or invalid preference, while an
earlier stored choice is preserved in the existing macOS `UserDefaults` domain
(unchanged since beta.4). Sharing remains explicit and sends to
`https://uploadimg.vercel.app`; finalized public links expire after 24 hours.
Candidates, result history and recovery capabilities are in memory and lost
on restart. Updates must block installation while images/uploads/unresolved
recovery remain, and restart requires confirmation. Feed failures remain
visible and can be retried through **Buscar actualizaciones**.

`SHA256SUMS` covers this DMG and its `provenance.json`. DMG packaging checks
cover metadata/icon/arm64/dependencies/ad-hoc signatures, UDZO integrity,
read-only mounted comparisons and exact-device detach. Packaging-time HEAD
is not a compiled-source claim; retain the separate source/bundle build
receipt. Updater archive/signature/manifest validation is a separate gate.

## Evidence and limits

The owner reported on 2026-11-01 that they tested updates and they work. This
is owner-reported device evidence, not an independently observed check, and it
does not identify the exact steps or cases.

Exact-source CI passed 178 checks and built the packages on macOS 26.6.2 arm64;
two opt-in integration tests were skipped. An independent review approved the
nine binary/update artifacts (signature, checksums, provenance, package contents
and DMG integrity); anonymous public downloads of all ten assets matched the
accepted files byte for byte, and the public DMG mounted read-only with its app
matching the updater app and the plist reporting 0.1.0 / build 5. The bundled
executable was not launched.

GUI updates, automatic restart, preferences through the GUI, Gatekeeper and
recovery after a real filesystem installation failure remain unverified.
No headless replacement test was run for beta.5; the beta.4 headless test and
earlier beta.2 device confirmation do not prove this version. Checks of this
exact installed app, download quarantine/Gatekeeper and another Mac are not
recorded. Intel, Windows/Linux and other macOS versions are unverified.
