#[path = "../update_trust.rs"]
mod update_trust;
fn main() {
    if let Err(message) = run() {
        eprintln!("VERIFY_FAILED={message}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 4 {
        return Err("Expected ARCHIVE SIG PUBLIC_KEY FULL_VERSION");
    }
    let version = args[3].to_str().ok_or("Invalid version")?;
    semver::Version::parse(version).map_err(|_| "Invalid SemVer")?;
    use std::io::Read;
    fn bounded(path: &std::ffi::OsStr, max: usize) -> Result<Vec<u8>, &'static str> {
        let file = std::fs::File::open(path).map_err(|_| "Input unavailable")?;
        let mut bytes = Vec::new();
        file.take(max as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Input unreadable")?;
        if bytes.len() > max {
            return Err("Input exceeds bound");
        }
        Ok(bytes)
    }
    let bytes = bounded(&args[0], update_trust::MAX_ARCHIVE_BYTES)?;
    let signature =
        String::from_utf8(bounded(&args[1], 8192)?).map_err(|_| "Invalid signature text")?;
    let key = String::from_utf8(bounded(&args[2], 4096)?).map_err(|_| "Invalid public key text")?;
    update_trust::verify(&bytes, &signature, &key, version)?;
    println!("VERIFIED_VERSION={version}");
    Ok(())
}
