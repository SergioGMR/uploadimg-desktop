fn main() {
    let desktop = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let version_path = desktop.parent().unwrap().join("VERSION");
    println!("cargo:rerun-if-changed={}", version_path.display());
    let version = std::fs::read_to_string(version_path).expect("VERSION must exist");
    let version = version.trim();
    semver::Version::parse(version).expect("VERSION must be full SemVer");
    println!("cargo:rustc-env=UPLOADIMG_VERSION={version}");
    let key_path = desktop.join("updater-public-key.txt");
    println!("cargo:rerun-if-changed={}", key_path.display());
    let key = match std::fs::read_to_string(key_path) {
        Ok(key) => key.trim().to_owned(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(_) => panic!("updater public key must be readable"),
    };
    let key = if key.contains(['\n', '\r']) {
        String::new()
    } else {
        key
    };
    println!("cargo:rustc-env=UPLOADIMG_UPDATER_PUBLIC_KEY={key}");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let native = root.join("native/macos");
        let output = root.join(".build/native");
        std::fs::create_dir_all(&output).expect("native output directory");
        let status = std::process::Command::new("xcrun")
            .args([
                "swiftc",
                "-swift-version",
                "5",
                "-emit-library",
                "-module-name",
                "UploadimgNative",
            ])
            .arg("-module-cache-path")
            .arg(root.join(".build/ModuleCache"))
            .arg(native.join("Images.swift"))
            .arg(native.join("SyntheticImage.swift"))
            .arg(native.join("Bridge.swift"))
            .arg(native.join("Upload.swift"))
            .arg("-o")
            .arg(output.join("libUploadimgNative.dylib"))
            .args([
                "-Xlinker",
                "-install_name",
                "-Xlinker",
                "@rpath/libUploadimgNative.dylib",
            ])
            .status()
            .expect("installed Xcode Swift compiler");
        assert!(status.success(), "native macOS bridge compile failed");
        for file in [
            "Images.swift",
            "SyntheticImage.swift",
            "Bridge.swift",
            "Upload.swift",
        ] {
            println!("cargo:rerun-if-changed={}", native.join(file).display());
        }
        println!("cargo:rustc-link-search=native={}", output.display());
        println!("cargo:rustc-link-lib=dylib=UploadimgNative");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_state",
            "generate_synthetic",
            "set_clipboard_enabled",
            "paste_image",
            "set_paused",
            "request_notifications",
            "candidate_action",
            "result_action",
            "updater_status",
            "updater_check",
            "updater_install",
        ]),
    ))
    .expect("Tauri local configuration must load");
}
