# Uploadimg Desktop beta for Apple Silicon

Open the DMG and drag **Uploadimg Desktop.app** onto **Applications**. Eject
the mounted disk, then open the app from Applications. It lives in the menu
bar; select **Abrir Uploadimg Desktop** to show its window and **Salir** to quit.

This is a local demonstration with simulated sharing. No image upload service
is connected; generated `example.invalid` links are fictitious. Clipboard
detection starts **OFF** on every launch. Enable it only when you intend to
detect copied images; sharing always requires an explicit action.

This beta preserves the exact tested debug build with an ad-hoc signature.
It has no Apple Developer ID signature or notarization. A downloaded copy may
be blocked by macOS; download/quarantine and installation behavior have not
been tested. This package provides no bypass of macOS protections.

Only Apple Silicon arm64 on macOS **26.6.2** was tested. The plist's 15.4 minimum
is metadata, not verified compatibility. Intel, Windows, Linux and other macOS
versions are unverified.

Distribution label: **0.1.0-beta.1**. The app retains version **0.1.0**, build **1**.
See `RELEASE-NOTES.txt` in the DMG for scope and verification limits.
