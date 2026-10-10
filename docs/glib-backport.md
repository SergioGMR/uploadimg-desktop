# GLib 0.18.5 backport

The Linux GTK3/WebKit dependency chain used by Tauri resolves Rust `glib` 0.18.5.
Replacing it with the officially fixed 0.20 series requires a compatible upstream
stack migration. This local backport preserves the existing 0.18.5 version and API.
It corrects the mutable C output argument in `VariantStrIter::impl_get`, shared by
`next`, `next_back`, `nth`, `nth_back` and `last`.

## Source and patch

`desktop/vendor/glib-0.18.5` contains the 121 files from the published
[`glib-0.18.5.crate` archive](https://static.crates.io/crates/glib/glib-0.18.5.crate),
with only two lines changed in `src/variant_iter.rs`.

| Source | SHA-256 |
| --- | --- |
| Registry archive, matching the original application lock checksum | `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5` |
| Original `src/variant_iter.rs` | `1fd02859333761c45321b32f28b24233446b97d0022a90d3a937ed162585b90e` |
| Backported `src/variant_iter.rs` | `a0f5ee8acb8faa089bcdfbc9a57372609fce7654026ccef7d9a224d05a654ccc` |

The archive's `.cargo_vcs_info.json` records upstream revision
`42b9caf98e03ded086362d9653ca58fe94dc8658`, path `glib`.
The registry cache marker `.cargo-ok` is excluded because it is not in the archive.
The upstream MIT `LICENSE` and `COPYRIGHT` remain intact; these apply to the vendored
crate and do not select a license for the application repository.

The change is the exact fix from
[gtk-rs-core PR #1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343/files):

```diff
-            let p: *mut libc::c_char = std::ptr::null_mut();
+            let mut p: *mut libc::c_char = std::ptr::null_mut();
@@
-                &p,
+                &mut p,
```

The workspace `[patch.crates-io]` selects this source. The application lock changes
only GLib's registry source/checksum to a local path; dependency versions remain
unchanged. Build source receipts include nonignored new files, so vendor, regression
and documentation are included in the exact source snapshot.

## Regression and evidence boundary

Run from the repository root on Linux with existing Rust, pkg-config and GLib
development dependencies and a populated offline Cargo cache:

```sh
bash desktop/scripts/test-glib.sh
```

The script installs nothing, uses the frozen independent harness lock, runs release
optimization and serializes test execution. `desktop/tests/glib-regression` is its
own workspace, so its dependencies do not join application tests. For security
validation, run in an OS sandbox with no network, an allowlisted empty environment,
read-only source/toolchain and bounded scratch, CPU, memory, processes and wall time.
Set `CARGO_HOME` and `CARGO_TARGET_DIR` to that scratch and use only existing cached
dependencies. The script itself does not create that sandbox.

The harness retains 11 cases, including four upstream array/string iterator cases.
It checks every affected method, UTF-8, empty and exhausted arrays, overflowing
indices, lengths, size hints and mixed traversal against Rust standard iterators.
Its test source SHA-256 is
`49a300824aef603ff248e4478ea9487424eebc2b3177b2fb38b758e576a5d2ff`.
Its explicit GLib and gio-sys features match the application's resolved Linux
feature graph: GLib `default`, `gio`, `gio_ffi` and `v2_58` through `v2_70`; gio-sys
0.18.1 has the same version features. Recheck this equality when the app lock changes.

The isolated precursor trial on Linux ARM64, Rust 1.92.0 and system GLib 2.74.6
observed RED (unmodified source SIGSEGV in release) followed by GREEN (the same
11 tests passed with this two-line patch), including application-matched features.
Local receipts/logs remain ignored under `desktop/.build/glib-validation-20261010`.
This records the precursor trial; validation of the integrated tree is separate.
The complete upstream unit suite was not run because its development dependencies
were unavailable offline. A harness result does not establish complete Linux app
compilation, GTK/WebKit compatibility, GUI runtime or Windows support. macOS excludes
GLib from its resolved application graph and needs separate non-regression checks.

## Advisory and maintenance

[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) lists the
officially patched range as `>=0.20.0`. This private backport retains `0.18.5`;
version-based scanners can continue to flag it. Do not describe the advisory as
cleared, suppress it or alter the package version to hide it. Review the vendored
source, provenance and regression evidence alongside the alert until a compatible
upstream migration replaces this backport.

Rollback is one unit: remove the workspace patch, restore GLib's original registry
source/checksum in the application lock and remove the vendor, harness, script and
associated documentation/changelog entries. That also restores the known unsound
code; rollback is not a remediation.
