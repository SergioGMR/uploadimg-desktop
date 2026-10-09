# Changelog

Changes follow [Semantic Versioning 2.0.0](https://semver.org/).
`VERSION` identifies the distribution; application manifests hold its numeric base.

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
