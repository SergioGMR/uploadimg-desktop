//! Opt-in official-plugin replacement of a disposable bundle, never the installed app.
#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri_plugin_updater::UpdaterExt;

fn isolated(root: &Path, bundle: &Path) -> Result<(), String> {
    let allowed = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".build/updater-runtime")
        .canonicalize()
        .map_err(|_| "Fixture root missing")?;
    let actual = root.canonicalize().map_err(|_| "Runtime root missing")?;
    if actual == allowed || !actual.starts_with(&allowed) {
        return Err(
            "Runtime root must be a run directory under desktop/.build/updater-runtime".into(),
        );
    }
    let expected = actual.join("old/UploadIMG.app");
    if bundle != expected
        || bundle
            .canonicalize()
            .map_err(|_| "Disposable bundle missing")?
            != expected
    {
        return Err("Only ROOT/old/UploadIMG.app is allowed; symlink escapes are rejected".into());
    }
    for relative in [
        "old",
        "old/UploadIMG.app",
        "old/UploadIMG.app/Contents",
        "old/UploadIMG.app/Contents/Info.plist",
        "old/UploadIMG.app/Contents/MacOS",
        "old/UploadIMG.app/Contents/MacOS/UploadimgDesktop",
    ] {
        if std::fs::symlink_metadata(actual.join(relative))
            .map_err(|_| "Fixture layout missing")?
            .file_type()
            .is_symlink()
        {
            return Err("Symlinked destination rejected".into());
        }
    }
    bundle_executable(&expected)?;
    let uid = std::fs::metadata(&actual)
        .map_err(|_| "Root metadata unavailable")?
        .uid();
    if uid == 0
        || uid
            != std::fs::metadata(env!("CARGO_MANIFEST_DIR"))
                .map_err(|_| "Project owner unavailable")?
                .uid()
    {
        return Err("Fixture owner must match the non-root project owner".into());
    }
    for directory in [&actual, &actual.join("old"), &expected] {
        let metadata = std::fs::metadata(directory).map_err(|_| "Fixture metadata unavailable")?;
        if metadata.uid() != uid || metadata.permissions().mode() & 0o200 == 0 {
            return Err(
                "Fixture must be owned and writable; no privileged installer allowed".into(),
            );
        }
    }
    Ok(())
}
fn bundle_executable(bundle: &Path) -> Result<PathBuf, String> {
    let output = std::process::Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleExecutable"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .map_err(|_| "Cannot read fixture CFBundleExecutable")?;
    if !output.status.success() {
        return Err("Fixture executable metadata is unavailable".into());
    }
    let name = String::from_utf8(output.stdout).map_err(|_| "Invalid executable metadata")?;
    if name.trim() != "UploadimgDesktop" {
        return Err("Fixture CFBundleExecutable must be UploadimgDesktop".into());
    }
    Ok(bundle.join("Contents/MacOS").join(name.trim()))
}
#[test]
fn runtime_fixture_uses_real_cf_bundle_executable_and_rejects_escape_name() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".build/updater-backend/harness-executable-test");
    std::fs::create_dir_all(fixture.join("Contents")).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("Info.plist"),
        fixture.join("Contents/Info.plist"),
    )
    .unwrap();
    assert_eq!(
        bundle_executable(&fixture).unwrap(),
        fixture.join("Contents/MacOS/UploadimgDesktop")
    );
    std::fs::write(fixture.join("Contents/Info.plist"),"<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleExecutable</key><string>../escape</string></dict></plist>").unwrap();
    assert!(bundle_executable(&fixture).is_err());
}
fn version_command(exe: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(exe);
    command.arg("--version");
    for name in [
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "DYLD_FALLBACK_FRAMEWORK_PATH",
    ] {
        command.env_remove(name);
    }
    command
}
#[test]
fn copied_bundle_version_probe_cannot_use_parent_native_library_paths() {
    let command = version_command(Path::new("fixture/Contents/MacOS/UploadimgDesktop"));
    let removed = command
        .get_envs()
        .filter_map(|(name, value)| if value.is_none() { Some(name) } else { None })
        .collect::<Vec<_>>();
    for name in [
        "DYLD_LIBRARY_PATH",
        "DYLD_FRAMEWORK_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "DYLD_FALLBACK_FRAMEWORK_PATH",
    ] {
        assert!(
            removed.contains(&std::ffi::OsStr::new(name)),
            "loader override inherited: {name}"
        );
    }
}
fn version(exe: &Path) -> String {
    let output = version_command(exe)
        .output()
        .expect("fixture --version early exit");
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
#[test]
fn runtime_fixture_rejects_installed_and_source_destinations() {
    assert!(isolated(
        Path::new("/Applications"),
        Path::new("/Applications/UploadIMG.app")
    )
    .is_err());
    assert!(isolated(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        Path::new(env!("CARGO_MANIFEST_DIR"))
    )
    .is_err());
}
#[test]
#[ignore = "Requires explicit signed bundles and a local HTTPS fixture; replaces only ROOT/old/UploadIMG.app"]
fn official_updater_replaces_only_disposable_signed_bundle() {
    let env = |name: &str| {
        std::env::var_os(name).expect("Explicit UPLOADIMG_RUNTIME fixture environment required")
    };
    let root = PathBuf::from(env("UPLOADIMG_RUNTIME_ROOT"))
        .canonicalize()
        .unwrap();
    let bundle = PathBuf::from(env("UPLOADIMG_RUNTIME_OLD_BUNDLE"));
    isolated(&root, &bundle).expect("isolated destination required");
    assert!(
        std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .starts_with(&root),
        "TMPDIR must be inside the runtime root"
    );
    // A MockRuntime never processes a privileged fallback's main-thread task.
    // A stalled installer therefore fails the entire dedicated fixture process,
    // without opening an authorization prompt or leaving an unbounded test.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        eprintln!("RUNTIME_FIXTURE_TIMEOUT=60s");
        std::process::exit(124);
    });
    let ca = PathBuf::from(env("UPLOADIMG_RUNTIME_CA"))
        .canonicalize()
        .unwrap();
    assert!(ca.starts_with(&root));
    let base = reqwest::Url::parse(&env("UPLOADIMG_RUNTIME_BASE_URL").to_string_lossy()).unwrap();
    assert_eq!(base.scheme(), "https");
    assert_eq!(base.host_str(), Some("localhost"));
    assert!(base.port().is_some());
    assert_eq!(base.path(), "/uploadimg-runtime/");
    assert!(
        base.query().is_none()
            && base.fragment().is_none()
            && base.username().is_empty()
            && base.password().is_none()
    );
    let next = env("UPLOADIMG_RUNTIME_NEW_VERSION")
        .to_string_lossy()
        .into_owned();
    let old = semver::Version::parse(env!("UPLOADIMG_VERSION")).unwrap();
    let new = semver::Version::parse(&next).unwrap();
    assert!(new.cmp_precedence(&old).is_gt());
    let exe = bundle_executable(&bundle).expect("validated CFBundleExecutable");
    let plist = bundle.join("Contents/Info.plist");
    let key = env!("UPLOADIMG_UPDATER_PUBLIC_KEY");
    assert!(!key.is_empty());
    assert_eq!(version(&exe), format!("UploadIMG {old}"));
    let before = std::fs::read(&exe).unwrap();
    let old_plist = std::fs::read(&plist).unwrap();
    assert!(
        before
            .windows(key.len())
            .any(|bytes| bytes == key.as_bytes()),
        "old key not embedded"
    );
    let cert = reqwest::Certificate::from_pem(&std::fs::read(ca).unwrap()).unwrap();
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().plugins.0.insert(
        "updater".into(),
        serde_json::json!({"pubkey":key,"requireSignedVersion":true}),
    );
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context)
        .expect("mock runtime without GUI");
    let started = Instant::now();
    for feed in ["bad-signature.json", "version-mismatch.json", "good.json"] {
        let updater = app
            .updater_builder()
            .target("darwin-aarch64")
            .executable_path(&exe)
            .endpoints(vec![base.join(feed).unwrap()])
            .unwrap()
            .timeout(Duration::from_secs(15))
            .version_comparator({
                let old = old.clone();
                move |_, offered| offered.version.cmp_precedence(&old).is_gt()
            })
            .configure_client({
                let cert = cert.clone();
                move |builder| {
                    builder
                        .https_only(true)
                        .add_root_certificate(cert.clone())
                        .redirect(reqwest::redirect::Policy::none())
                }
            })
            .build()
            .unwrap();
        let offered = tauri::async_runtime::block_on(updater.check())
            .expect("local HTTPS check")
            .expect("new fixture version");
        assert_eq!(offered.download_url.scheme(), "https");
        assert_eq!(offered.download_url.host_str(), Some("localhost"));
        assert_eq!(offered.download_url.port(), base.port());
        assert!(offered
            .download_url
            .path()
            .starts_with("/uploadimg-runtime/"));
        let mut offered = offered;
        offered.timeout = Some(Duration::from_secs(15));
        let downloaded = tauri::async_runtime::block_on(offered.download(|_, _| {}, || {}));
        if feed != "good.json" {
            assert!(
                downloaded.is_err(),
                "invalid signature/version passed plugin trust"
            );
            assert_eq!(
                std::fs::read(&exe).unwrap(),
                before,
                "failed download changed app"
            );
            assert_eq!(
                std::fs::read(&plist).unwrap(),
                old_plist,
                "failed download changed plist"
            );
        } else {
            assert_eq!(offered.version, next);
            let bytes = downloaded.expect("valid signed-version download");
            assert!(bytes.len() <= 256 * 1024 * 1024);
            isolated(&root, &bundle).unwrap();
            // The fixture deadline covers any unexpected privileged fallback.
            offered
                .install(bytes)
                .expect("official replacement of writable disposable bundle");
            assert_eq!(bundle_executable(&bundle).unwrap(), exe);
            assert_eq!(version(&exe), format!("UploadIMG {next}"));
            assert!(
                std::fs::read(&exe)
                    .unwrap()
                    .windows(key.len())
                    .any(|bytes| bytes == key.as_bytes()),
                "new key not embedded"
            );
            assert_ne!(
                std::fs::read(&plist).unwrap(),
                old_plist,
                "new build plist unchanged"
            );
            assert_ne!(
                std::fs::metadata(&exe).unwrap().permissions().mode() & 0o111,
                0,
                "executable mode lost"
            );
            assert!(bundle
                .join("Contents/Frameworks/libUploadimgNative.dylib")
                .is_file());
        }
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "fixture deadline exceeded"
        );
    }
    println!("OFFICIAL_RUNTIME_REPLACEMENT={old}->{new}");
}
