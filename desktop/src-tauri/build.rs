fn main() {
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
        for file in ["Images.swift", "SyntheticImage.swift", "Bridge.swift"] {
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
        ]),
    ))
    .expect("Tauri local configuration must load");
}
