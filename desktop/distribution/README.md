# UploadIMG beta for Apple Silicon

Verify the DMG against its `SHA256SUMS`, open it and drag **UploadIMG.app**
onto **Applications**. Eject the disk and open UploadIMG from Applications.
Close the previous app first. UploadIMG lives in the menu bar; select
**Abrir UploadIMG** to show its window and **Salir** to quit.

This is **0.1.0-beta.4**, short version **0.1.0**, build **4**,
debug profile. Beta.3 was the first updater-capable beta: install manually
when upgrading from beta.1/beta.2, which do not contain an updater. The
executable's `--version` option reports the full distribution version. See `RELEASE-NOTES.txt` for
check limits. The release was published on 2026-10-10 UTC; all nine anonymous
public downloads matched the accepted CI artifacts. These repository notes were
updated after publication; the published packages retain their tagged-source notes.
From beta.3, use **Buscar actualizaciones** and confirm the offered installation.

Clipboard detection starts **OFF** with a fresh or invalid preference. Explicit
ON/OFF is remembered across normal restart and replacement of this app with
the same bundle identifier; an earlier stored choice is preserved. Enable it
only when you intend to detect copied images. Pause is temporary and preserves
that choice. Restoration uses a fresh baseline without admitting old clipboard
content; denied or unavailable reads persist OFF. Only the boolean preference
is stored in macOS `UserDefaults`.
**Compartir** sends the selected image explicitly to
`https://uploadimg.vercel.app`; returned links expire after 24 hours. Candidates,
result history and recovery state are in memory and lost on restart. Finish
or explicitly discard pending images/recovery before confirming an update.

The bundle is ad-hoc signed, without Developer ID or Apple notarization.
macOS may block a downloaded copy. After verifying origin and checksum, use
its opening option in **System Settings → Privacy & Security** if you choose
to proceed. This package does not bypass macOS protections. Updater archive
signing is separate and does not establish Gatekeeper acceptance.

GUI updates, automatic restart, Gatekeeper and recovery after a real filesystem
installation failure, preferences through the GUI and another Mac remain unverified.
Exact-source CI passed 147 checks on macOS **26.6.2 arm64**. A separate headless
test replaced disposable published beta.3 with the exact beta.4 CI bundle and
verified signature/version rejection. That does not prove GUI runtime.
The plist minimum 15.4 is
metadata; Intel, Windows/Linux and other macOS versions remain unverified.
