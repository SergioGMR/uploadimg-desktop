use crate::events::{Event, EventQueue, ImageEvent};
use std::sync::{Mutex, OnceLock};

static QUEUE: OnceLock<Mutex<EventQueue>> = OnceLock::new();
fn queue() -> &'static Mutex<EventQueue> {
    QUEUE.get_or_init(|| Mutex::new(EventQueue::default()))
}
pub fn drain() -> Vec<Event> {
    queue().lock().map(|mut q| q.drain()).unwrap_or_default()
}
pub fn available() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::ffi::{c_char, CString};
    type Callback = unsafe extern "C" fn(
        i32,
        u64,
        u64,
        i32,
        u32,
        u32,
        *const u8,
        usize,
        *const u8,
        usize,
        *const u8,
        usize,
    );
    extern "C" {
        fn uig_start(callback: Callback);
        fn uig_configure(
            enabled: i32,
            paused: i32,
            generation: u64,
            user_change: i32,
            revision: u64,
        );
        fn uig_generate();
        fn uig_paste();
        fn uig_request_notifications();
        fn uig_notify(id: *const c_char, result: i32, revision: u64);
        fn uig_sync_ids(ids: *const c_char, revision: u64);
        fn uig_result_operation(id: *const c_char, action: i32);
        fn uig_monotonic_millis() -> u64;
    }

    // The callback is a process-lifetime function pointer, without a heap context.
    // Each Swift buffer is synchronously copied here; all stored data is Rust-owned.
    unsafe extern "C" fn receive(
        kind: i32,
        generation: u64,
        lifecycle: u64,
        code: i32,
        width: u32,
        height: u32,
        original: *const u8,
        original_len: usize,
        thumbnail: *const u8,
        thumbnail_len: usize,
        text: *const u8,
        text_len: usize,
    ) {
        if text_len > 128 || (text_len > 0 && text.is_null()) {
            return;
        }
        let text = if text_len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(text, text_len)
        };
        let Ok(text) = std::str::from_utf8(text) else {
            return;
        };
        let event = if kind == 1 {
            if original.is_null()
                || thumbnail.is_null()
                || original_len == 0
                || original_len > 10_000_000
                || thumbnail_len < 24
                || thumbnail_len > 1_100_000
                || text.len() != 64
                || !text
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || width == 0
                || height == 0
                || width > 8192
                || height > 8192
                || u64::from(width) * u64::from(height) > 16_000_000
            {
                return;
            }
            let mut digest = [0u8; 32];
            for (n, byte) in digest.iter_mut().enumerate() {
                let Ok(value) = u8::from_str_radix(&text[n * 2..n * 2 + 2], 16) else {
                    return;
                };
                *byte = value;
            }
            Event::Image(ImageEvent {
                generation,
                lifecycle,
                source: code,
                width,
                height,
                digest,
                original: std::slice::from_raw_parts(original, original_len).to_vec(),
                thumbnail: std::slice::from_raw_parts(thumbnail, thumbnail_len).to_vec(),
            })
        } else {
            Event::Control {
                kind,
                generation,
                lifecycle,
                code,
                text: text.into(),
            }
        };
        if let Ok(mut q) = queue().lock() {
            q.push(event);
        }
    }
    pub fn start() {
        unsafe { uig_start(receive) }
    }
    pub fn configure(
        enabled: bool,
        paused: bool,
        generation: u64,
        user_change: bool,
        revision: u64,
    ) {
        unsafe {
            uig_configure(
                enabled as i32,
                paused as i32,
                generation,
                user_change as i32,
                revision,
            )
        }
    }
    pub fn generate() {
        unsafe { uig_generate() }
    }
    pub fn paste() {
        unsafe { uig_paste() }
    }
    pub fn request_notifications() {
        unsafe { uig_request_notifications() }
    }
    pub fn notify(id: &str, result: bool, revision: u64) {
        if let Ok(id) = CString::new(id) {
            unsafe { uig_notify(id.as_ptr(), result as i32, revision) }
        }
    }
    pub fn sync_ids(ids: &[String], revision: u64) {
        if let Ok(json) = serde_json::to_string(ids) {
            if let Ok(text) = CString::new(json) {
                unsafe { uig_sync_ids(text.as_ptr(), revision) }
            }
        }
    }
    pub fn result_operation(id: &str, action: i32) {
        if let Ok(id) = CString::new(id) {
            unsafe { uig_result_operation(id.as_ptr(), action) }
        }
    }
    pub fn monotonic_millis() -> u64 {
        unsafe { uig_monotonic_millis() }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn ffi_copies_borrowed_buffers_and_rejects_non_ascii_digest_without_unwinding() {
            drain();
            let mut original = vec![1u8, 2, 3];
            let mut thumb = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
            thumb.extend(2u32.to_be_bytes());
            thumb.extend(2u32.to_be_bytes());
            let digest = "ab".repeat(32);
            unsafe {
                receive(
                    1,
                    9,
                    4,
                    1,
                    2,
                    2,
                    original.as_ptr(),
                    original.len(),
                    thumb.as_ptr(),
                    thumb.len(),
                    digest.as_ptr(),
                    digest.len(),
                );
            }
            original[0] = 99;
            thumb[0] = 0;
            let events = drain();
            assert_eq!(events.len(), 1);
            let Event::Image(copied) = &events[0] else {
                panic!("copied image event")
            };
            assert_eq!((copied.generation, copied.lifecycle), (9, 4));
            assert_eq!(copied.original, [1, 2, 3]);
            assert_eq!(copied.thumbnail[0], 137);
            assert_eq!(copied.digest, [0xab; 32]);
            let invalid = "é".repeat(32);
            unsafe {
                receive(
                    1,
                    10,
                    4,
                    1,
                    2,
                    2,
                    original.as_ptr(),
                    original.len(),
                    thumb.as_ptr(),
                    thumb.len(),
                    invalid.as_ptr(),
                    invalid.len(),
                );
            }
            assert!(drain().is_empty());
        }
    }
}
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
mod unavailable {
    pub fn start() {}
    pub fn configure(_: bool, _: bool, _: u64, _: bool, _: u64) {}
    pub fn generate() {}
    pub fn paste() {}
    pub fn request_notifications() {}
    pub fn notify(_: &str, _: bool, _: u64) {}
    pub fn sync_ids(_: &[String], _: u64) {}
    pub fn result_operation(_: &str, _: i32) {}
    pub fn monotonic_millis() -> u64 {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_millis() as u64
    }
}
#[cfg(not(target_os = "macos"))]
pub use unavailable::*;
