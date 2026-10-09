# Proposed public image-sharing API v1

This is a documented contract for a future public adapter, not a description of the existing local `/api/images` implementation or a deployed endpoint. The machine-readable source is [uploadimg-v1.openapi.json](uploadimg-v1.openapi.json), OpenAPI 3.1. The web/API repository owns releases; desktop pins a released schema, fixtures and digest.

## Request and ready result

`POST /api/v1/images` accepts a raw JPEG, PNG or static WebP body. Send matching `Content-Type`, mandatory `Content-Length` and a freshly generated 256-bit random lowerhex64 `Idempotency-Key`. The request sends no filename or source metadata. Input limits are 10,000,000 encoded bytes, 16,000,000 pixels and 8,192 pixels per edge. Declared and decoded MIME must agree; animations are rejected.

Ready success is 201 for a newly published image or 200 for replay. Example values are illustrative:

```json
{
  "id": "0123456789abcdef0123456789abcdef0123456789abcdef",
  "url": "/i/0123456789abcdef0123456789abcdef0123456789abcdef",
  "readyAt": 1791417600000,
  "expiresAt": 1791504000000,
  "width": 1280,
  "height": 720,
  "bytes": 123456,
  "inputBytes": 345678,
  "format": "webp"
}
```

Times are server-authoritative epoch milliseconds; `expiresAt - readyAt` is exactly 86,400,000. Output WebP is at most 5,000,000 bytes and 1,920 pixels per edge. The response appears only after durable object and metadata publication; 201 never means queued. `Content-Location` identifies `/api/v1/images/{id}`. `url` is the relative human-facing `/i/{id}` route, resolved against the configured web origin only; ID and URL must agree. Schema constraints cannot express every cross-field invariant, so fixtures/runtime validation must enforce these relationships.

## Receipt and idempotency semantics

The key is an anonymous secret receipt capability, not authentication of a person or app. Server uniqueness is global across replicas. Store a server-side HMAC/hash rather than the raw key, exclude it from logs, and keep it out of URLs. A deployment threat review must resolve hash strategy, key/rotation handling and secure client storage before rollout.

1. Perform framing and bounded admission before buffering/decode. A shared provisional processing claim can reserve key ownership/capacity before reading; a same-key processing request may receive 409 `upload_in_progress` plus `Retry-After` without full body receipt. No changed-body comparison is promised in that case.
2. For admitted requests that reach comparison, read under the cap, validate declared versus actual length, and compute a final fingerprint of actual bytes plus normalized allowlisted MIME. Do not trust a client digest or compare based only on length/MIME.
3. Atomically commit/compare the final fingerprint in shared durable storage before associating input with a final outcome. Fully computed different input returns 409 `idempotency_conflict`; matching processing input returns 409 `upload_in_progress`. A provisional claim is not proof of an input fingerprint.
4. Decode/process under bounded worker limits, commit publication and store the immutable ready result. Matching ready replay returns the original ID, sizes and timestamps without a new image or extended expiry.
5. After a durable terminal-failure commit, POST/GET receipt errors include `receiptState: "failed"`. Matching replays return the recorded failure; clients never POST that key again. If persistence fails, omit the marker and treat the outcome as unknown.

Framing/admission errors 400/411/413/415/429 are never stored as terminal receipts. The exhaustive recorded terminal pairs are 422 `invalid_image`, 507 `storage_full`, 503 `storage_unavailable`, and 500 `internal_error`. The marker is allowed only on upload POST/receipt GET and only after durable commit; HTTP status alone never proves persistence. GET receipt 422/507 requires it; GET 500/503 may be an unmarked lookup error or a marked recorded failure. An absent marker or receipt is not proof that no upload was sent.

Retain reservation/result records for 48 hours from first reservation. Before every ready replay/result, revalidate availability: expired ready images return 410 `upload_expired` while retained; blocked/deleted images before expiry return uniform 404 `not_found`. Never return stale 200/201 as though available or resurrect an image. After ledger purging, recognition of that historical key is not guaranteed and GET returns 404. Do not promise perpetual idempotency or 410 after purge. Clients never retry POST more than 10 minutes after the first accepted attempt, and only while original immutable bytes remain retained; a fresh explicit share uses a new key.

If a worker crashes after object write but before publication commit, the object is not accessible and is an orphan cleanup candidate. Recover stale reservations through ownership/fencing or an equivalent atomic mechanism: prove the prior worker cannot publish before admitting another worker. Never release concurrency early on a client disconnect. Recovery resolves one ready result or a recorded terminal failure without raw durable input; inaccessible orphan cleanup cannot expose a second image.

`GET /api/v1/uploads/result` sends the same key header without image bytes. It returns 200 currently ready, 202 `{ "status": "processing" }` with `Retry-After`, 404 unknown/invalid capability or unavailable former image, 410 expired success while its record exists, or its recorded terminal error. Throttle receipt lookup as well as uploads. Before first POST, desktop must persist the key and accepted-operation metadata safely without raw bytes; persistence failure prevents transfer. Restart reconciles through this endpoint, never automatically repeating POST. Server timeout values and retry intervals remain provider-dependent; the synchronous adapter uses bounded admission and 429/503 backpressure instead of an unbounded job queue.

## Read and report

| Operation | Response and guarantee |
| --- | --- |
| `GET /api/v1/images/{id}` | Optimized WebP; access check before metadata or bytes. Optional `download=1` requests attachment with a generic safe filename. |
| `HEAD /api/v1/images/{id}` | Same access decision/representation headers as GET, no body, including error responses. |
| Missing, expired or blocked image | Uniform 404 without a reason oracle. No direct public object-store URL. |
| `POST /api/v1/images/{id}/report` | JSON `{ "reason": "inappropriate" \| "copyright" \| "other" }`, maximum 1,024 bytes; 202 `{ "reported": true }` if accessible, otherwise 404. Rate-limited. |

Report acceptance does not promise review completion, block an image or change expiry. Repeated reports MUST preserve the first accepted reason (matching current local behavior); exact retention and operator policy are public-release gates. Report request content type is `application/json` and unknown/malformed reason is rejected. Read/report 500/503 describes that operation's failure, carries no receipt marker and does not require querying an upload receipt. The `/i/{id}` web route is not a JSON API operation.

All representation and JSON responses use `Cache-Control: private, no-store` and `Referrer-Policy: no-referrer`. Image `Content-Type` is `image/webp`; downloads use `Content-Disposition: attachment; filename="image.webp"`. HEAD is bodyless; its representation media type is documented in the description/extension, since OpenAPI ignores `Content-Type` in response `headers`. See [OpenAPI 3.1 Response Object](https://spec.openapis.org/oas/v3.1.0.html#response-object). Access denial at expiry is independent of asynchronous object deletion. Clock synchronization and finite processing/recovery bounds must fit the ledger lifetime before rollout.

## Error handling

Error JSON is `{ "error": "machine_code", "message": "Human-readable text" }`, optionally with `receiptState: "failed"` under the terminal-receipt rules above. Clients use `error` and the marker, not message wording; unknown errors fail safely without inventing success. Messages expose no stack, paths, image contents or other uploads. HEAD is the sole bodyless variant. Illustrative 503 cases:

```json
{"error":"storage_unavailable","message":"Storage is unavailable."}
```

The unmarked error does not prove a terminal receipt. After a durable terminal commit:

```json
{"error":"storage_unavailable","message":"Storage is unavailable.","receiptState":"failed"}
```

| HTTP | Codes / use |
| --- | --- |
| 400 | `malformed_request`, `invalid_idempotency_key`, `invalid_length` |
| 404 | `not_found` (image/report or blocked/deleted ready replay/result); `upload_unknown` (receipt, including invalid key or purged record) |
| 409 | `idempotency_conflict`; `upload_in_progress` with `Retry-After` |
| 410 | `upload_expired` while a ready receipt record remains |
| 411 | `length_required` |
| 413 | `too_large` |
| 415 | `unsupported_media_type` |
| 422 | `invalid_image`, `invalid_report` |
| 429 | `rate_limited`, `busy`; `Retry-After` required |
| 500 | `internal_error`; unmarked upload/receipt error leaves outcome unknown; read/report failure is separate |
| 503 | `unavailable`, `storage_unavailable`; `Retry-After` required |
| 507 | `storage_full` |

`Retry-After` is a positive integer number of seconds. Honor it within the client window; it never overrides a terminal marker or authorizes POST. Receipt polling can use bounded retries. HTTP timeouts/offline/disconnects convey no success or failure proof. No link is shown until a ready response validates.

## Client recovery

| Observed upload/receipt result | State and permitted next action |
| --- | --- |
| Validated 200/201 ready | Ready; show validated absolute link and preserve expiry. |
| `receiptState: "failed"` | TerminalFailed; never rePOST the same key. A new share requires reselection, consent and a new key. |
| Timeout/disconnect or unmarked server failure (500/503; POST 507 if terminal persistence failed) | Unknown; consult GET receipt. A status alone does not mean a durably recorded terminal failure. |
| GET 202 / unmarked GET 500/503 | Remain Unknown; bounded lookup retries, no POST. |
| GET 404 `upload_unknown`, original bytes retained, first attempt less than 10 minutes ago | Explicit UserRetry may POST the same immutable bytes/key, after backoff. Never automatic POST. |
| GET 404 `upload_unknown` after restart, without bytes, or outside retry window | Reselect, consent and a new key; never retry old bytes/key. |
| 404 `not_found` / 410 `upload_expired` | Unavailable; never retry old key. A new share requires reselection, consent and a new key. |
| Permanently invalid 400/411/413/415/422, or 409 `idempotency_conflict` | Correct/reselect input rather than identical retry; new share requires consent and a new key. |
| 409 `upload_in_progress` / unmarked admission 429 | Consult receipt with bounded backoff; only the eligible `upload_unknown` row permits UserRetry. |

## Release gates and compatibility

Before public use, implement shared atomic receipts, durable private storage/metadata, proxy-aware rate/capacity controls, decoder size/time/memory budgets, expiry denial, bounded retention of objects/reports/receipt records, operator abuse handling and HTTPS. Native requests may have no Origin; that absence is not an authentication bypass. CORS is not identity or rate control. No production provider, domain, throughput or cost capacity is selected by this contract.

OpenAPI describes the target surface and invariants; it does not establish schema-validator, provider, integration or runtime proof. Both repositories need independent compatibility CI with released positive/negative fixtures, including changed-body conflicts, lost-response reconciliation, expiry and malicious result URLs. Additive v1 fields are allowed; breaking changes require v2. Existing local guards must remain intact until separately authorized migration.

See [the product definition](../desktop-companion.md) and [implementation acceptance map](../desktop-implementation-plan.md).
