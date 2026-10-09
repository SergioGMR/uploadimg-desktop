# UploadIMG 0.1.0-beta.2 — local, unpublished

This development version adds the UploadIMG app name and macOS icon and
connects explicit sharing to the existing `https://uploadimg.vercel.app`
prepare/private-upload/finalize flow. Finalized `/i/` links expire after
24 hours. An uncertain finalization can be retried with the same in-memory
request without uploading the image again; restart loses recovery state.
Clipboard detection starts OFF; dedicated-folder detection remains unavailable.

The package contains **UploadIMG.app**, an Applications shortcut and
installation notes. It packages an existing debug build without compiling,
re-signing or launching it. The application short version is **0.1.0**,
build **2**. Signing is ad-hoc; there is no Developer ID or notarization.

The new installer is `UploadIMG-0.1.0-beta.2-macos-arm64.dmg` under
`desktop/.build/distribution/0.1.0-beta.2/`. The published beta.1 installer
and tag are preserved. Run `bash desktop/scripts/package-dmg.sh` only after
the current source and bundle checks finish. Existing outputs are refused;
use a new output subdirectory for a retry.

`provenance.json` records packaging-time HEAD and dirty status, source digest,
bundle hashes and DMG hash. Its null `compiled_source_commit` explicitly
avoids attributing a prebuilt binary to the observed commit. `SHA256SUMS`
covers the installer and provenance. Bundle identity/signatures/architecture,
UDZO verification, read-only mounted comparisons and exact-device detach
are packaging checks, not runtime evidence.

Validation of the exact beta.2 source, bundle, real upload and DMG remains
pending until its local receipts are recorded. Earlier macOS 26.6.2 arm64
observations do not prove this version's runtime. Installed-app launch,
another Mac, downloaded quarantine/Gatekeeper, Intel, Windows, Linux and
other macOS versions are NOT_RUN unless later evidence states otherwise.
No release publication or download verification is claimed here.

Rollback for this packaging unit removes only the packager/helper/tests,
distribution instructions and newly generated beta.2 outputs. It leaves
application behavior and all published beta.1 bytes untouched.
