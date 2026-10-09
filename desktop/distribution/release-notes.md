# Uploadimg Desktop 0.1.0-beta.1

Local beta packaging of the existing macOS arm64 debug bundle that the user
tested. This is not an optimized release build. Packaging does not compile,
re-sign, launch or modify the source app. The app retains CFBundle version
0.1.0 (build 1); 0.1.0-beta.1 is the distribution label only.

The app provides a menu-bar window, image candidates and previews, explicit
Ignore/Share actions, and simulated results. Uploads and public API integration
are not implemented. `example.invalid` result links are fictitious. Clipboard
detection starts OFF each process; dedicated-folder detection is unavailable.

The package includes the app, an Applications shortcut and installation notes.
It retains the existing ad-hoc signatures without Developer ID or notarization.
There is no paid Apple account or distribution-signing claim.

Packaging checks cover exact source/mounted file hashes, bundle versions,
arm64 binaries, system/bundled dylib dependencies and rpaths, deep/strict
signature verification, UDZO integrity, read-only mounting, the Applications
shortcut, instruction files, clean unmounting and SHA256SUMS.

Only macOS 26.6.2 arm64 has local device evidence. LSMinimumSystemVersion 15.4
does not establish compatibility with macOS 15.4. Intel, Windows, Linux and
other macOS versions are unverified. Installation, launch of the installed
copy, another Mac and downloaded quarantine/Gatekeeper behavior are NOT_RUN.
Packaging checks do not establish those runtime results.

Traceability of the prebuilt tested bundle (these values are hashes, not commit
identifiers; packaging does not infer source history for that bundle):

- Source snapshot identity: `d7426aa8cba880ccecfbc0d7fcee7a1693dcda86e4265d7baa4b5a0e43042308`.
- Tested app executable SHA256: `2641d50045d82b2a13a51286d77c8b322903e1cc17be8f4cc09ea8f0f19db386`.

To package again with existing local tools, run
`bash desktop/scripts/package-dmg.sh`. Outputs are written under
`desktop/.build/distribution/`. The script refuses to overwrite prior output;
pass a new subdirectory of that directory as its optional argument to rerun.
The root `VERSION` supplies the strict SemVer label and its base must match the
bundle's CFBundleShortVersionString.
It checks the retained source-snapshot and app identity, so a different build
needs a separately reviewed distribution boundary. DMG container bytes may
vary between runs; verify the generated SHA256SUMS for each artifact.

Rollback boundary: remove only the new packaging script, distribution notes,
`desktop/evidence/dmg-*.txt` receipts and `desktop/.build/distribution/` outputs.
The existing app and source-snapshot remain untouched. Application tests are
N/A for this packaging-only change; its runtime boundary is DMG verification
and read-only content comparison, without launching the app.
