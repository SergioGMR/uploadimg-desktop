# Implement the desktop companion in verifiable phases

This plan turns the [product definition](desktop-companion.md) and [proposed API v1](api/uploadimg-v1.md) into future work. Documentation is the authorized outcome now. No desktop repository, dependencies, application implementation, credentials, public service or release is created by this plan.

## Work ownership and dependency path

| Phase | Owner / repository | Deliverable | Depends on | Exit evidence |
| --- | --- | --- | --- | --- |
| D1 Native feasibility | Desktop executor / future desktop repo | macOS file/provenance, pasteboard, native notification/action and tray/settings bridge probes, with fake transport | Explicit later repo/tooling authorization | Target-device observations and supported-version/CPU matrix; action behavior, privacy prompts, denial, memory/latency measurements. |
| A1 Public adapter | Web/API executor / uploadimg | Separate v1 adapter, durable private storage, atomic receipts and fixtures | API/provider/storage decisions and corresponding authorization; can proceed independently of D1 | RED→GREEN behavior tests; replica/crash/expiry integration evidence; existing local loopback guard still enforced. |
| D2 Shell and candidates | Desktop executor | Rust state core, Tauri settings/tray, native source adapters, bounded candidates/config | D1 pass or explicitly accepted product revision | Core/state tests, device source/setup/pause proof, measured resource bounds. |
| D3 Consent and recovery | Desktop executor | Notification actions, native transport, receipt/credential handling, bounded result history | D2; A1 released for real integration, fake transport otherwise | Consent race/duplicate/unknown-result tests and native action verification. |
| I1 Integration and compatibility | Separate reviewers plus executors in each repo | Contract artifacts pinned, real client/server scenarios, adversarial URL/state checks | A1 and D3 | Exact schema digest and independently run compatibility checks on both repos; local proof explicitly distinct from public environment proof. |
| R1 Distribution | Desktop release owner and web operator | Signed/notarized app plus selected public service | I1; separate distribution/provider/credential approvals | Packaging install/upgrade/uninstall evidence on advertised platform matrix, public expiry/abuse/operations proof. |

A public API release precedes a desktop release using it. D1 does not require infrastructure deployment. Keep repositories independently releasable; no cross-repo source imports or lockstep version requirement. The parent assigns disjoint file owners; serialize shared storage, generated contract artifacts and integration resources. A reviewer in a separate thread inspects actual diffs without editing them. Corrections need proportional re-review after writes stop.

## Feasibility decisions before implementation commitments

- Prove screenshot provenance using supported metadata or explicit dedicated-folder setup. Establish safe stable-write detection, selected-directory access and sandbox/bookmark implications.
- Prove native UserNotifications **Compartir/Ignorar** and **Copiar enlace/Abrir enlace** actions on macOS, validated absolute URL in the result body, full URL in tray/app, truncation/lock-screen preview settings, grouped result delivery, action ID integrity, dismissal and restart handling. Do not promise in-place updates. Tauri's mobile-only Actions API does not meet this proof.
- Confirm provisional macOS 15.4 minimum, SDK APIs, CPU architectures, permission behavior and distribution mode. Do not advertise support from documentation alone.
- Measure bounded TIFF/raw decode→PNG conversion and static image validation. Final decoder limits/timeouts and API timeout values need measurements/provider decisions.
- Decide provider/domain/durable store, shared atomic ledger and recovery fencing, trusted-proxy/rate strategy, abuse operator workflow, data retention and credential storage. The contract records requirements, not an authorization to configure these resources.

If notification actions cannot support the confirmed flow, freeze that phase, record evidence and present a popover alternative for explicit product review. If provenance is unavailable in a general directory, require explicit dedicated-folder setup or revise detection scope; do not use filenames as proof.

## Acceptance coverage

All cases below are future checks, not executed tests. Use unit tests for the pure state/core contract, integration tests for server durability/concurrency, and actual macOS devices for OS interfaces. Observe RED before implementing behavioral changes, then GREEN and justified refactoring. Documentation checks are proportional schema/content/reference/load checks rather than application tests.

| ID | Requirement / check | Acceptance evidence | Phase |
| --- | --- | --- | --- |
| C01 | Clipboard off by default and after disabling | Instrumented adapter proves no content inspections/reads; persisted toggle and unapproved clipboard-candidate removal | D1, D2 |
| C02 | OS pasteboard denial/read failure | Real denial/prompt behavior: source stops, truthful UI, explicit paste fallback; no recurring consent prompts or text/HTML/URL/file-reference reads | D1, D2 |
| C03 | Folder baseline and screenshot provenance | Existing files/start/resume/wake emit no retrospective candidate; general-image file is not classified by filename alone | D1, D2 |
| C04 | Stable writes, symlinks and file races | Partial/renamed/replaced/escaped/nonregular input rejected or safely snapshotted; original remains intact | D1, D2 |
| C05 | Immutable candidate / clipboard replacement | Share after path/clipboard replacement sends the original candidate bytes; stale action never reads current source | D2, D3 |
| C06 | Duplicate events and Share actions | Across sources same pixels within 30s deduplicate; simultaneous/duplicate Share causes one accepted operation; recopy outside window can notify | D2, D3 |
| C07 | Encoded/decoded/animated bounds | Limits at/below/above 10MB, 16MP, 8192px; animations rejected; raw conversion and one-worker memory/time measured; overflow stays at 3/30MB | D1, D2, A1 |
| C08 | Consent and TTL | Notification body/Ignore/dismiss/Focus/TTL never upload; Share atomically accepts before 10min; stale candidate cannot upload | D1, D3 |
| C09 | Offline / unknown outcome | Timeout/unmarked server failure remains Unknown, including POST 507 if terminal persistence failed; GET 202/unmarked 500/503 remain Unknown; no fabricated link or auto POST. Only GET upload_unknown + original retained bytes + <10min permits explicit same-key UserRetry; terminal marker never does | A1, D3 |
| C10 | Restart and receipt recovery | Persist secure key/metadata before first POST; save failure prevents transfer; bytes vanish, stale Share invalid; receipt recovers ready/processing/marked terminal without image transfer; unknown after restart needs reselection/consent/new key | A1, D3 |
| C11 | Pause / source disable races | New candidates stop and matching unapproved candidates vanish; accepted operation can finish; resume establishes baseline | D2, D3 |
| C12 | Notifications denied / Focus | Tray/popover exposes bounded pending status; no inferred consent or auto upload; native action limits documented | D1, D3 |
| C13 | URL and action input validation | Wrong scheme/host/path, mismatched ID, external notification paths/URLs and unknown IDs rejected; Copy/Open only explicit validated actions | D3, I1 |
| C14 | API framing and decoded MIME | 400/411/413/415/429 never recorded terminal; permanently invalid input corrected/reselected, never identical retry; recorded terminal pairs only 422 invalid_image, 507 storage_full, 503 storage_unavailable, 500 internal_error, with marker only after commit | A1, D3 |
| C15 | Shared idempotency across replicas | Bounded pre-read admission/provisional ownership; processing409 may precede body receipt; actual bytes+MIME fingerprint before final comparison/commit; fully compared changed input409; ready replay identical | A1, I1 |
| C16 | Crash and reservation recovery | Crash after object write exposes no image; fenced recovery produces one ready result or recorded error; disconnect does not release slot prematurely | A1 |
| C17 | Expiry and receipt purge | GET/HEAD404 at 24h before cleanup; retained successful receipt/replay410; blocked/deleted before expiry404, never stale success; after 48h purge receipt404 allowed; no POST retry >10min | A1, D3, I1 |
| C18 | Copy/Open and result privacy | Visible validated absolute result URL + actions, full URL in tray/app; respect OS truncation/previews without link-secrecy promise; no auto Copy/Open, thumbnails or raw history; max20 metadata entries, links until expiry, unresolved credentials at most48h without eviction to admit new upload | D1, D3 |
| C19 | Contract compatibility | Both repos pin schema/fixtures; additive fields ignored, required/cross-field invariants checked; both 503 marker variants covered, receipt 422/507 requires marker, read/report prohibits marker; HEAD bodyless and media type described without ignored Content-Type header | A1, I1 |
| C20 | Report and public abuse handling | Bounded reason-only request, inaccessible404, accepted202, repeated reports MUST preserve first reason, rate limits; read/report 500/503 needs no upload receipt; report never auto-blocks/extends expiry; operator path demonstrated | A1, R1 |
| C21 | Public deployment and distribution | HTTPS, durable private objects, receipt retention, decoder budgets, trusted proxy/rate controls, no sensitive logs, signed/notarized install proof on advertised matrix | R1 |

## Reviewable work units

1. Native probe evidence and explicit go/no-go decisions, with fake transport only.
2. API schema/fixture release and server adapter behavior/durability. Keep local ingress and public admission policies separate.
3. Desktop pure core/preferences and native detection, then consent/action bridge.
4. Transport/receipt recovery and credential lifecycle, then end-to-end compatibility.
5. Distribution and public operational proof after separate approvals.

Each future unit reports files, exact revisions/schema digest, RED/GREEN commands actually run, failures/skips, native test matrix, independent review result and remaining deployment proof. Passing local mocks cannot establish clipboard, notification, provider or production behavior. A stored setting is not an OS permission; a signed package is not proof of runtime source access.

## Future authorization checkpoints

Creating the separate desktop repository, installing Rust/Tauri/SDK dependencies, implementing code, selecting/configuring storage/provider/domain, accessing signing or deployment credentials, publishing contract/releases, committing/pushing and deploying each require their applicable later authorization. These checkpoints do not require revisiting routine actions already explicitly authorized in a future session.

Current definition acceptance is: readable linked documents, parseable OpenAPI 3.1 JSON, consistent bounds/flows/ownership, and independent review. Formal schema-validator and native/runtime verification are not established by this documentation task.
