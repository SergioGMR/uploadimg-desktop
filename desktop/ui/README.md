# Desktop interface

Bundled static Spanish UI; no frontend build or npm dependencies. Load `index.html` inside the Tauri application with `withGlobalTauri: true`. A normal browser shows an inactive native-app notice and never fabricates candidates or results.

`app.mjs` calls only `window.__TAURI__.core.invoke`. `bridge.mjs` validates command names/arguments, serializes requests, coalesces timer polls and discards pre-action polling snapshots. Failed actions keep their visible error until a successful explicit action. `state.mjs` accepts the bounded simulated state contract and strips unsupported thumbnails. The DOM preserves cards across countdown updates and inserts strings with `textContent`; result URLs are text, with Copy/Open delegated by opaque ID.

Commands: `get_state`, `generate_synthetic`, `set_clipboard_enabled({ enabled })`, `paste_image`, `set_paused({ paused })`, `request_notifications`, `candidate_action({ id, action: preview|share|ignore })`, `result_action({ id, action: copy|open })`. Each returns the documented `UiState`; permission requests and Share/Copy/Open always follow clicks. Poll interval: one second, without overlapping native invokes.

The shell requires CSP to allow local scripts/styles and `img-src data:` for Rust-prepared PNG thumbnails. No inline JavaScript/styles, remote assets, web clipboard calls or frontend network requests. Folder monitoring is shown as pending; other platform availability comes from native state. System color preference controls light/dark styling.

Thumbnails accept PNG dimensions up to 512 pixels per edge and at most 1,100,000 decoded PNG bytes. The exact maximum data URL length is 1,466,690 characters including the MIME prefix; the adapter checks base64 padding to enforce the byte cap and decodes only the header for PNG/dimension validation.

Run `node --test desktop/ui/tests/*.test.mjs` from the repository root. RED and GREEN receipts are in `evidence/`; they establish adapter/state behavior, not native clipboard, permissions, WebView rendering or platform runtime support. Actual Tauri GUI validation belongs to the parent after the native build.

Rollback boundary: remove `desktop/ui/` as one interface unit; backend ownership and the original native probe are separate. No commit or remote delivery is included.
