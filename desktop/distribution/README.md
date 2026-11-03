# UploadIMG beta for Apple Silicon

Verify the DMG against its `SHA256SUMS`, open it and drag **UploadIMG.app**
onto **Applications**. Eject the disk and open UploadIMG from Applications.
Close the previous app first. UploadIMG lives in the menu bar; select
**Abrir UploadIMG** to show its window and **Salir** to quit.

This is **0.1.0-beta.5**, short version **0.1.0**, build **5**,
debug profile. Beta.3 was the first updater-capable beta: install manually
when upgrading from beta.1/beta.2, which do not contain an updater. The
executable's `--version` option reports the full distribution version. See
`RELEASE-NOTES.txt` for check limits. The release was published on 2026-11-03
UTC; all ten anonymous public downloads matched the accepted files. These
repository notes were updated after publication; the published packages retain
their tagged-source notes. From beta.3 or beta.4, use **Buscar actualizaciones**
and confirm the offered installation.

Clipboard detection starts **OFF** with a fresh or invalid preference. Explicit
ON/OFF is remembered across normal restart and replacement of this app with
the same bundle identifier; an earlier stored choice is preserved. Enable it
only when you intend to detect copied images. Pause is temporary and preserves
that choice. Restoration uses a fresh baseline without admitting old clipboard
content; denied or unavailable reads persist OFF. Only the boolean preference
is stored in macOS `UserDefaults`. Files and folders copied in Finder are not
offered for sharing, even though Finder also places their icon on the clipboard.
**Compartir** sends the selected image explicitly to
`https://uploadimg.vercel.app`; returned links expire after 24 hours. Candidates,
result history and recovery state are in memory and lost on restart. Finish
or explicitly discard pending images/recovery before confirming an update.

The bundle is ad-hoc signed, without Developer ID or Apple notarization.
macOS may block a downloaded copy. After verifying origin and checksum, use
its opening option in **System Settings → Privacy & Security** if you choose
to proceed. This package does not bypass macOS protections. Updater archive
signing is separate and does not establish Gatekeeper acceptance.

The owner reported on 2026-11-01 that they tested updates and they work; this
is owner-reported, not an independently observed check, and does not identify the exact steps or cases.
Otherwise GUI updates, automatic restart, preferences through the GUI, Gatekeeper,
recovery after a real filesystem installation failure and another Mac remain
unverified. Exact-source CI passed 178 checks on macOS **26.6.2 arm64**. No
headless replacement test was run for beta.5; the one recorded for beta.4 does
not prove this version. The plist minimum 15.4 is metadata; Intel,
Windows/Linux and other macOS versions remain unverified.

## License and source

Original UploadIMG Desktop material is licensed under GNU GPL version 3 only
(GPL-3.0-only), without warranty. New builds include the full LICENSE, the project
COPYRIGHT and THIRD-PARTY-NOTICES (licenses of the Rust dependencies) unchanged in
UploadIMG.app/Contents/Resources; DMG and updater checks verify those resources.
`THIRD-PARTY-NOTICES` lists 13 crates (objc2 family, block2, alloc-stdlib,
selectors, dispatch2) without license text because the published crates ship no
license file; a later version is planned to address it.
Third-party components retain their own licenses and copyright notices,
including the vendored GLib MIT files in the source tree.
Recipients of new binary distributions must also receive access to the exact
Corresponding Source under the GPL. For beta.5 it is the tag source archive
`UploadIMG-0.1.0-beta.5-source.tar.gz`, attached to the same Release as the DMG;
dependency sources come from crates.io as pinned by `desktop/Cargo.lock`
(`cargo fetch --locked` in `desktop/`). The published archive matched the tag
archive after download. The already published beta.4 packages and
their tagged installation notes are unchanged.
