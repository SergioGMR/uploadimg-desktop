# Changelog

Changes follow [Semantic Versioning 2.0.0](https://semver.org/).
`VERSION` identifies the distribution; application manifests hold its numeric base.

## [0.1.0-beta.4] - 2026-10-10

### Changed

- macOS remembers explicit clipboard detection ON/OFF across normal restarts and application replacement with the same bundle identifier, using the existing `UserDefaults` preference domain. A fresh or invalid preference starts OFF; an earlier stored choice is preserved.
- Pause remains temporary and does not change that preference. Restored detection establishes a fresh baseline and does not admit old clipboard content; denied or unavailable reads persist OFF.
- Only the boolean `clipboardIntent` is stored. Images, previews, links and upload recovery remain in memory.

### Availability

- Published macOS arm64 beta, short version 0.1.0 and bundle build 4, from source `1faedbca1cdc90669d2ebaafffbeaf770989fd3c` on 2026-10-10 UTC. Exact-source CI passed 147 checks on macOS 26.6.2 arm64; two opt-in integration tests were skipped.
- Nine reviewed artifacts passed signature/checksum/provenance/package checks; anonymous public downloads matched every accepted artifact byte for byte.
- A separate headless test replaced a disposable copy of published beta.3 with the exact beta.4 CI bundle and rejected an altered signature and signed-version mismatch without changing the old app. This is distinct from the earlier beta.4 fixture experiment.
- GUI updates, automatic restart, preferences through the GUI, Gatekeeper and recovery after a real filesystem installation failure remain unverified.
- Debug bundle with Apple ad-hoc signing; no Developer ID or notarization. Windows/Linux, Intel and other macOS versions remain unverified.

## [0.1.0-beta.3] - 2026-10-10

### Added

- macOS arm64 automatic-update integration and signed updater packaging.
- Manual GitHub Actions build workflow producing a DMG and signed update artifacts; release publication remains a separate action.

### Fixed

- Vendored the official two-line GLib `VariantStrIter` backport for the Linux dependency graph, with an independent release-mode regression harness. The crate remains 0.18.5; version-based RUSTSEC-2024-0429 alerts may persist. Full Linux application runtime remains unverified.

### Availability

- First updater-capable macOS arm64 beta. Install it manually when upgrading from beta.1/beta.2, which do not contain an updater.
- Local pre-release checks passed 137 macOS regressions and 11 isolated Linux GLib regressions; the latter do not verify the complete Linux application.
- A headless macOS harness verified real signed beta.3→beta.4 replacement of a disposable bundle and signature/version rejection; beta.4 is a local fixture only.
- GUI updates, automatic restart, Gatekeeper and recovery after a real filesystem failure remain unverified.
- The final source build, CI execution, release/feed publication and public downloads require their own release receipts; the local checks above do not establish those results.
- Debug bundle with Apple ad-hoc signing; updater archive signing is separate. No Developer ID or notarization; Windows/Linux are unverified.

## [0.1.0-beta.2]

### Added

- Explicit real image sharing through UploadIMG's upload service, with public links expiring after 24 hours.
- Recovery of uncertain finalization using the same in-memory upload capability.
- High-resolution application icon based on the website's image glyph.

### Changed

- Installed application name is now UploadIMG; bundle identifier remains stable.

### Availability

- macOS arm64 debug installer: `UploadIMG-0.1.0-beta.2-macos-arm64.dmg`.
- The user tested the local beta.2 DMG and reported that it works; individual GUI cases were not recorded.
- Local ad-hoc signing, without Developer ID or Apple notarization. Windows/Linux runtime is NOT_RUN.

## [0.1.0-beta.1]

### Added

- First macOS Apple Silicon desktop beta with a menu-bar image workflow.
- Explicit image preview, Ignore and simulated Share, with fictitious results.
- Opt-in clipboard image detection and explicit Paste; detection starts OFF.
- Local pause/resume, bounded in-memory candidates and synthetic demo images.
- Version checks, issue templates and a manual DMG release procedure.

### Availability

- Runtime evidence covers macOS 26.6.2 arm64 only. This beta packages the tested debug bundle.
- Real uploads, Windows/Linux installers, Apple distribution signing and notarization are unavailable.
