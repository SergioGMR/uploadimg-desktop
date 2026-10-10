# UploadIMG 0.1.0-beta.4

This published macOS arm64 beta remembers the explicit clipboard detection
choice after normal restart and replacement with the same bundle identifier.
Beta.3 was the first updater-capable version; install manually from beta.1/beta.2,
which do not contain an updater. From beta.3, use **Buscar actualizaciones**
and confirm the offered installation.
Published on 2026-10-10 UTC from source `1faedbca1cdc90669d2ebaafffbeaf770989fd3c`.
These repository notes were updated afterward; published packages and their
tagged-source notes remain unchanged.

The DMG contains **UploadIMG.app**, an Applications shortcut and installation
notes. It packages the reviewed custom debug bundle without compiling,
re-signing or launching it. Application short version: **0.1.0**; build **4**.
The bundle uses an Apple ad-hoc signature with no Developer ID/notarization.
The updater archive uses a separate dedicated Tauri/minisign signature.

Clipboard detection starts OFF with a fresh or invalid preference, while an
earlier stored choice is preserved in the existing macOS `UserDefaults` domain.
Explicit ON/OFF survives normal restart and app replacement. Pause is temporary
and preserves the choice; restored detection starts from a fresh baseline and
does not admit old clipboard content. Denied/unavailable reads persist OFF.
Only the boolean `clipboardIntent` is stored. Sharing remains explicit and sends to
`https://uploadimg.vercel.app`; finalized public links expire after 24 hours.
Candidates, result history and recovery capabilities are in memory and lost
on restart. Updates must block installation while images/uploads/unresolved
recovery remain, and restart requires confirmation. Feed failures remain
visible and can be retried through **Buscar actualizaciones**.

Exact-source CI passed 147 checks and built the packages on macOS 26.6.2 arm64;
two opt-in integration tests were skipped. Nine artifacts passed signature,
checksum, provenance and package verification, including DMG read-only mounting
and the bundled executable's version. Anonymous public downloads matched all
nine accepted artifacts byte for byte.

A separate headless official-updater test over localhost HTTPS replaced a
disposable copy of published beta.3 with the exact beta.4 CI bundle. Installed
bytes and permissions matched; altered signature and signed-version mismatch
were rejected without changing the previous app, and test resources were cleaned.
This is separate from the historical beta.4 fixture experiment.

`SHA256SUMS` covers this DMG and its `provenance.json`. DMG packaging checks
cover metadata/icon/arm64/dependencies/ad-hoc signatures, UDZO integrity,
read-only mounted comparisons and exact-device detach. Packaging-time HEAD
is not a compiled-source claim; retain the separate source/bundle build
receipt. Updater archive/signature/manifest validation is a separate gate.

GUI updates, automatic restart, Gatekeeper and recovery after a real filesystem
installation failure, preferences through the GUI and another Mac remain unverified.
Historical beta.2 device confirmation does
not prove this version. Intel, Windows/Linux and other macOS versions are
unverified. Headless replacement and download integrity do not prove GUI runtime.
