# UploadIMG beta for Apple Silicon

Verify the DMG against its `SHA256SUMS`, open it and drag **UploadIMG.app**
onto **Applications**. Eject the disk and open UploadIMG from Applications.
Close the previous app first. UploadIMG lives in the menu bar; select
**Abrir UploadIMG** to show its window and **Salir** to quit.

This is a **0.1.0-beta.3 local, unreleased candidate**, short version **0.1.0**,
build **3**, debug profile. Published beta.1/beta.2 artifacts remain separate.
The first updater-capable version requires manual installation; no working
public updater feed is claimed here. See `RELEASE-NOTES.txt` for check limits.

Clipboard detection starts **OFF** each launch. Enable it only when you intend
to detect copied images. **Compartir** sends the selected image explicitly to
`https://uploadimg.vercel.app`; returned links expire after 24 hours. Candidates,
result history and recovery state are in memory and lost on restart. Finish
or explicitly discard pending images/recovery before confirming an update.

The bundle is ad-hoc signed, without Developer ID or Apple notarization.
macOS may block a downloaded copy. After verifying origin and checksum, use
its opening option in **System Settings → Privacy & Security** if you choose
to proceed. This package does not bypass macOS protections. Updater archive
signing is separate and does not establish Gatekeeper acceptance.

Earlier device evidence covers macOS **26.6.2** Apple Silicon. This exact
candidate's installed launch, update/recovery, another Mac and download
quarantine/Gatekeeper require separate checks. The plist minimum 15.4 is
metadata; Intel, Windows/Linux and other macOS versions remain unverified.
