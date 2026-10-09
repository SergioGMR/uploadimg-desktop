# Changelog

Changes follow [Semantic Versioning 2.0.0](https://semver.org/).
`VERSION` identifies the distribution; application manifests hold its numeric base.

## [0.1.0-beta.3] - Unreleased

### Added

- Local macOS arm64 automatic-update integration and signed updater packaging.
- Manual GitHub Actions build workflow producing a DMG and signed update artifacts; release publication remains a separate action.

### Availability

- Local development version, not a published download. The first updater-capable version requires manual installation.
- A headless macOS harness verified real signed beta.3→beta.4 replacement of a disposable bundle and signature/version rejection; beta.4 is a local fixture only.
- GUI updates, automatic restart, Gatekeeper, recovery after a real filesystem failure, workflow execution and release/feed publication remain unverified.
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
