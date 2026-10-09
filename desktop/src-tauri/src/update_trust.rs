//! Verification uses the same official minisign implementation as the updater.
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use semver::Version;

pub const MAX_ARCHIVE_BYTES: usize = 256 * 1024 * 1024;
fn decoded(encoded: &str) -> Result<String, &'static str> {
    let bytes = STANDARD
        .decode(encoded.trim())
        .map_err(|_| "Invalid base64 trust data")?;
    String::from_utf8(bytes).map_err(|_| "Invalid trust text")
}
pub fn valid_key(encoded: &str) -> bool {
    encoded.len() <= 4096
        && decoded(encoded)
            .ok()
            .and_then(|text| PublicKey::decode(&text).ok())
            .is_some()
}
pub fn signed_version_matches(comment: &str, expected: &str) -> bool {
    let versions: Vec<_> = comment
        .split('\t')
        .filter_map(|field| field.strip_prefix("version:"))
        .collect();
    if versions.len() != 1 {
        return false;
    }
    match (
        Version::parse(versions[0].trim_start_matches('v')),
        Version::parse(expected),
    ) {
        (Ok(signed), Ok(expected)) => signed == expected,
        _ => false,
    }
}
pub fn verify(bytes: &[u8], signature: &str, key: &str, version: &str) -> Result<(), &'static str> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err("Archive exceeds 256 MiB");
    }
    if signature.len() > 8192 || key.len() > 4096 {
        return Err("Trust data exceeds limit");
    }
    if !valid_key(key) {
        return Err("Invalid public key");
    }
    let key = PublicKey::decode(&decoded(key)?).map_err(|_| "Invalid public key")?;
    let signature = Signature::decode(&decoded(signature)?).map_err(|_| "Invalid signature")?;
    key.verify(bytes, &signature, true)
        .map_err(|_| "Signature verification failed")?;
    if !signed_version_matches(signature.trusted_comment(), version) {
        return Err("Signed version mismatch or missing");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authenticated_comment_must_bind_exact_full_version() {
        assert!(signed_version_matches(
            "timestamp:1\tversion:0.1.0-beta.3",
            "0.1.0-beta.3"
        ));
        for text in [
            "timestamp:1",
            "version:0.1.0-beta.2",
            "version:0.1.0",
            "version:0.1.0-beta.3\tversion:0.1.0-beta.2",
        ] {
            assert!(!signed_version_matches(text, "0.1.0-beta.3"));
        }
    }
    #[test]
    fn official_valid_legacy_signature_cannot_authorize_inflated_feed_version() {
        // Public fixture from minisign-verify 0.2.5; no private signing material.
        let key = STANDARD.encode("untrusted comment: minisign public key E7620F1842B4E81F\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3");
        let signature = STANDARD.encode("untrusted comment: signature from minisign secret key\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: timestamp:1555779966\tfile:test\nQtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==");
        assert!(valid_key(&key));
        let pk = PublicKey::decode(&decoded(&key).unwrap()).unwrap();
        let sig = Signature::decode(&decoded(&signature).unwrap()).unwrap();
        pk.verify(b"test", &sig, true).unwrap();
        assert_eq!(
            verify(b"test", &signature, &key, "99.0.0"),
            Err("Signed version mismatch or missing")
        );
        assert_eq!(
            verify(b"Test", &signature, &key, "99.0.0"),
            Err("Signature verification failed")
        );
        assert!(!valid_key(""));
        assert!(!valid_key(&STANDARD.encode("invalid key")));
    }
}
