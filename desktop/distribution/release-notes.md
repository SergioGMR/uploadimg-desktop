# UploadIMG 0.1.0-beta.3 — local, unreleased

This development candidate adds macOS arm64 automatic-update integration
and signed updater artifact tooling. The first updater-capable version must
be manually installed: published beta.1/beta.2 do not contain an updater.
No release or channel feed publication is established by these notes.

The DMG contains **UploadIMG.app**, an Applications shortcut and installation
notes. It packages the reviewed custom debug bundle without compiling,
re-signing or launching it. Application short version: **0.1.0**; build **3**.
The bundle uses an Apple ad-hoc signature with no Developer ID/notarization.
The updater archive uses a separate dedicated Tauri/minisign signature.

Clipboard detection starts OFF. Sharing remains explicit and sends to
`https://uploadimg.vercel.app`; finalized public links expire after 24 hours.
Candidates, result history and recovery capabilities are in memory and lost
on restart. Updates must block installation while images/uploads/unresolved
recovery remain, and restart requires confirmation. Feed failures remain
visible until real signed releases and channel manifests are published.

`SHA256SUMS` covers this DMG and its `provenance.json`. DMG packaging checks
cover metadata/icon/arm64/dependencies/ad-hoc signatures, UDZO integrity,
read-only mounted comparisons and exact-device detach. Packaging-time HEAD
is not a compiled-source claim; retain the separate source/bundle build
receipt. Updater archive/signature/manifest validation is a separate gate.

Checks of this exact beta.3 installed app, update/recovery, CI execution,
download quarantine/Gatekeeper and another Mac remain pending unless separate
receipts record their execution. Historical beta.2 device confirmation does
not prove this version. Intel, Windows/Linux and other macOS versions are
unverified. No source commit, release publication or download verification
is claimed by generating this package.
