# UploadIMG desktop interface

Bundled Spanish UI with no frontend build or npm dependencies. Load
`index.html` in Tauri with `withGlobalTauri: true`. A normal browser shows an
inactive native-app notice and never fabricates candidates or results.

**Compartir** sends the selected image to `https://uploadimg.vercel.app`
through the backend and creates a public link that expires after 24 hours.
Detection, preview and polling never authorize an upload. Clipboard detection
starts OFF; folder monitoring is unavailable. Name/title and the image glyph
match UploadIMG branding.

## State and actions

`state.mjs` validates `simulation: false`, bounded pending/results arrays,
32-character lowercase hexadecimal candidate IDs and 48-character result IDs.
Every result must have `fictitious: false`, a URL exactly equal to
`https://uploadimg.vercel.app/i/<result ID>`, and an expiration of at most
86,400 seconds. Unsupported thumbnail sources are stripped.

`upload` is null or `{ id: <candidate ID>, status: "uploading" | "recoverable" }`.
An active upload disables all candidate Share buttons. The separate upload
panel remains available after the candidate leaves the pending list.
**Recuperar enlace** is enabled only for `recoverable` and invokes
`candidate_action({ id: upload.id, action: "share" })`: the backend finalizes
the existing ticket without uploading the image again. Recovery state is
in memory and is lost on restart.

Commands: `get_state`, `generate_synthetic`,
`set_clipboard_enabled({ enabled })`, `paste_image`, `set_paused({ paused })`,
`request_notifications`, `candidate_action({ id, action: preview|share|ignore })`
and `result_action({ id, action: copy|open })`. Copy/Open receive only opaque
result IDs. URLs are inserted with `textContent`; the UI does not navigate
to caller-provided URLs or make network requests.

`bridge.mjs` validates command arguments, serializes invokes, coalesces
one-second polls and discards snapshots started before an action. Failed
actions preserve their visible error through polls until a successful
explicit action. Cards/buttons retain their DOM identity during countdown
updates. Permission requests and sharing/recovery/copy/open require clicks.

## Checks and limits

```sh
node --test desktop/ui/tests/*.test.mjs
```

Tests cover the state/IPC guards, request ordering, error persistence and
upload/recovery controls using a small DOM fixture. They are not evidence
of WebView rendering, native clipboard/permissions or real network uploads.
The parent validates the built native application separately.

The UI keeps the existing CSP/IPC contract: local scripts/styles, bounded
PNG data thumbnails, no frontend networking or remote assets. Thumbnail
limits remain 512 pixels per edge and 1,100,000 decoded PNG bytes; base64
padding enforces the byte cap and only the PNG header is decoded for guards.
System color preference controls light/dark styling.

Rollback of this slice restores the UI files and matching tests; backend
upload ownership is a separate unit. No commit or remote delivery is included.
