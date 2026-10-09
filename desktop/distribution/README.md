# UploadIMG beta for Apple Silicon

Verify the DMG against its `SHA256SUMS`, open it and drag **UploadIMG.app**
onto **Applications**. Eject the disk and open UploadIMG from Applications.
The app lives in the menu bar; select **Abrir UploadIMG** to show its window
and **Salir** to quit.

Clipboard detection starts **OFF** at each launch. Enable it only when you
intend to detect copied images. **Compartir** is an explicit action that
sends the selected image to `https://uploadimg.vercel.app`; returned links
expire after 24 hours. Candidates and recovery state remain in memory and
are lost on restart.

This is the local, unpublished **0.1.0-beta.2** package: application version
**0.1.0**, build **2**, debug profile. The earlier beta.1 installer remains a
separate published artifact. See `RELEASE-NOTES.txt` for verification limits.

The bundle has a local ad-hoc signature, without Developer ID or Apple
notarization. macOS may block a downloaded copy. After verifying its origin
and checksum, use the opening option macOS offers in **System Settings →
Privacy & Security** if you choose to proceed. This package does not bypass
macOS protections.

Earlier device evidence covers macOS **26.6.2**, Apple Silicon arm64. Checks
of this exact new package, installed launch, another Mac and downloaded
quarantine/Gatekeeper behavior must be recorded separately. The plist's
15.4 minimum is metadata; Intel, Windows, Linux and other macOS versions
have no verified compatibility claim.
