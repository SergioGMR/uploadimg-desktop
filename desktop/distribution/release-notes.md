# UploadIMG 0.1.0-beta.4 candidate

This prepared macOS arm64 beta remembers the explicit clipboard detection
choice after normal restart and replacement with the same bundle identifier.
Beta.3 was the first updater-capable version; install manually from beta.1/beta.2,
which do not contain an updater. These notes do not establish release or channel
publication, exact-source CI execution or public download verification.

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

Historical beta.3 pre-release evidence records 137 passing macOS regressions and
11 isolated Linux GLib regressions. A headless macOS harness performed real
signed beta.3→beta.4 replacement of a disposable bundle and rejected invalid
signatures and a signed-version mismatch. That beta.4 fixture is separate from
this candidate and does not validate its runtime, graphical confirmation,
restart or recovery after an actual filesystem installation failure.

`SHA256SUMS` covers this DMG and its `provenance.json`. DMG packaging checks
cover metadata/icon/arm64/dependencies/ad-hoc signatures, UDZO integrity,
read-only mounted comparisons and exact-device detach. Packaging-time HEAD
is not a compiled-source claim; retain the separate source/bundle build
receipt. Updater archive/signature/manifest validation is a separate gate.

GUI updates, automatic restart, Gatekeeper and recovery after a real filesystem
installation failure remain unverified for this candidate. Independent
exact-source/artifact review and download verification remain required.

Checks of this exact beta.4 installed app, update/recovery, CI execution,
download quarantine/Gatekeeper and another Mac remain pending unless separate
receipts record their execution. Historical beta.2 device confirmation does
not prove this version. Intel, Windows/Linux and other macOS versions are
unverified. No source commit, release publication or download verification
is claimed by generating this package.
