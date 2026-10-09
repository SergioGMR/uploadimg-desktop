use super::*;
const ID: &str = "0123456789abcdef0123456789abcdef";
const RESULT_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef";
fn image(digest: u8) -> Image {
    Image::new(vec![1, 2, 3], [digest; 32], 2, 2).unwrap()
}
fn admit(core: &mut Core) {
    core.admit(ID.into(), image(1), Source::Synthetic, 0, false)
        .expect("valid image admission");
}

#[test]
fn preview_ignore_and_invalid_actions_do_not_share() {
    let mut core = Core::default();
    admit(&mut core);
    assert!(matches!(
        core.act(ID, CandidateAction::Preview, "", 1),
        Ok(Effect::Preview(_))
    ));
    assert!(core.results().is_empty());
    assert_eq!(core.pending().len(), 1);
    assert!(matches!(
        core.act(ID, CandidateAction::Ignore, "", 2),
        Ok(Effect::Ignored)
    ));
    assert!(core.act(ID, CandidateAction::Share, RESULT_ID, 3).is_err());
    assert!(core.results().is_empty());
    assert!(core.pending().is_empty());
}
#[test]
fn share_accepts_frozen_owned_bytes_exactly_once() {
    let mut original = vec![1, 2, 3];
    let immutable = Image::new(original.clone(), [1; 32], 2, 2).unwrap();
    let mut core = Core::default();
    core.admit(ID.into(), immutable, Source::Synthetic, 0, false)
        .unwrap();
    original[0] = 99;
    let Ok(Effect::Shared(sent)) = core.act(ID, CandidateAction::Share, RESULT_ID, 1) else {
        panic!("explicit Share missing")
    };
    assert_eq!(sent.bytes(), &[1, 2, 3]);
    assert!(core.act(ID, CandidateAction::Share, RESULT_ID, 2).is_err());
    assert!(
        core.results().is_empty(),
        "consent must not fabricate a successful upload"
    );
    assert!(core.pending().is_empty());
}
#[test]
fn ttl_rejects_stale_and_monotonic_clock_cannot_roll_back() {
    let mut core = Core::default();
    admit(&mut core);
    core.expire(599_999);
    assert_eq!(core.pending().len(), 1);
    core.expire(600_000);
    assert!(core.pending().is_empty());
    assert!(core.act(ID, CandidateAction::Share, RESULT_ID, 0).is_err());
    core.admit(ID.into(), image(2), Source::Synthetic, 0, false)
        .unwrap();
    assert_eq!(core.pending()[0].created_at, 600_000);
}
#[test]
fn clipboard_defaults_off_explicit_paste_does_not_enable_and_disable_purges() {
    let mut core = Core::default();
    assert!(!core.clipboard_enabled());
    assert_eq!(
        core.admit(ID.into(), image(1), Source::Clipboard, 0, false),
        Err(Error::Disabled)
    );
    core.admit(ID.into(), image(1), Source::Clipboard, 0, true)
        .unwrap();
    assert!(!core.clipboard_enabled());
    core.set_clipboard_enabled(false);
    assert!(core.pending().is_empty());
    core.set_clipboard_enabled(true);
    core.admit(ID.into(), image(2), Source::Clipboard, 1, false)
        .unwrap();
    core.set_clipboard_enabled(false);
    assert!(core.pending().is_empty());
}
#[test]
fn pause_purges_without_upload_and_resume_has_no_old_candidates() {
    let mut core = Core::default();
    admit(&mut core);
    core.set_paused(true);
    assert!(core.pending().is_empty());
    assert!(core.results().is_empty());
    assert_eq!(
        core.admit(ID.into(), image(2), Source::Synthetic, 1, false),
        Err(Error::Paused)
    );
    core.set_paused(false);
    assert!(core.pending().is_empty());
}
#[test]
fn duplicate_pixels_across_sources_expire_at_30_seconds() {
    let mut core = Core::default();
    core.set_clipboard_enabled(true);
    admit(&mut core);
    core.act(ID, CandidateAction::Ignore, "", 1).unwrap();
    assert_eq!(
        core.admit(ID.into(), image(1), Source::Clipboard, 29_999, false),
        Err(Error::Duplicate)
    );
    core.admit(ID.into(), image(1), Source::Clipboard, 30_000, false)
        .unwrap();
}
#[test]
fn failed_retry_never_releases_a_newer_fingerprint_or_an_unrelated_image() {
    let mut core = Core::default();
    admit(&mut core);
    core.act(ID, CandidateAction::Share, RESULT_ID, 1).unwrap();
    core.expire(30_000);
    let newer = "c".repeat(32);
    core.admit(newer.clone(), image(1), Source::Synthetic, 30_001, false)
        .unwrap();
    core.allow_failed_retry([1; 32], 0);
    core.act(&newer, CandidateAction::Ignore, "", 30_002)
        .unwrap();
    assert_eq!(
        core.admit(ID.into(), image(1), Source::Clipboard, 30_003, true),
        Err(Error::Duplicate)
    );
    core.admit(ID.into(), image(2), Source::Synthetic, 30_004, false)
        .unwrap();
    core.allow_failed_retry([1; 32], 30_001);
    core.act(ID, CandidateAction::Ignore, "", 30_005).unwrap();
    assert_eq!(
        core.admit(ID.into(), image(2), Source::Clipboard, 30_006, true),
        Err(Error::Duplicate)
    );
}
#[test]
fn image_bounds_reject_empty_encoded_and_decoded_overflow_accept_boundaries() {
    for (bytes, w, h) in [
        (vec![], 1, 1),
        (vec![0; 10_000_001], 1, 1),
        (vec![1], 8193, 1),
        (vec![1], 4001, 4000),
        (vec![1], 0, 1),
        (vec![1], u32::MAX, u32::MAX),
    ] {
        assert!(Image::new(bytes, [1; 32], w, h).is_err());
    }
    assert!(Image::new(vec![0; 10_000_000], [1; 32], 4000, 4000).is_ok());
    assert!(Image::new(vec![1], [1; 32], 8192, 1).is_ok());
}
#[test]
fn admission_revalidates_metadata_of_an_existing_image_value() {
    let mut invalid = image(1);
    invalid.width = 8193;
    let mut core = Core::default();
    assert_eq!(
        core.admit(ID.into(), invalid, Source::Synthetic, 0, false),
        Err(Error::InvalidImage)
    );
    assert!(core.pending().is_empty());
}
#[test]
fn pending_cap_is_three_and_thirty_mb() {
    let mut core = Core::default();
    for n in 0..3 {
        let id = format!("{n:032x}");
        let image = Image::new(vec![n; 10_000_000], [n; 32], 1, 1).unwrap();
        core.admit(id, image, Source::Synthetic, 0, false).unwrap();
    }
    assert_eq!(
        core.pending()
            .iter()
            .map(|p| p.image.bytes().len())
            .sum::<usize>(),
        30_000_000
    );
    assert_eq!(
        core.admit(ID.into(), image(7), Source::Synthetic, 0, false),
        Err(Error::Capacity)
    );
}
#[test]
fn invalid_ids_and_invalid_result_do_not_consume_consent() {
    let mut core = Core::default();
    assert_eq!(
        core.admit("../../path".into(), image(1), Source::Synthetic, 0, false),
        Err(Error::InvalidId)
    );
    admit(&mut core);
    assert!(core.act(ID, CandidateAction::Share, "wrong", 1).is_err());
    assert_eq!(core.pending().len(), 1);
    assert!(core
        .act("unknown", CandidateAction::Share, RESULT_ID, 1)
        .is_err());
    assert!(core.results().is_empty());
}
#[test]
fn result_body_is_passive_and_copy_open_are_explicit_exact_url_actions() {
    let mut core = Core::default();
    admit(&mut core);
    core.act(ID, CandidateAction::Share, RESULT_ID, 1).unwrap();
    core.record_result(
        RESULT_ID.into(),
        format!("https://uploadimg.vercel.app/i/{RESULT_ID}"),
        1,
        86_400_000,
    )
    .unwrap();
    assert!(matches!(
        core.result_action(RESULT_ID, ResultAction::Preview, 2),
        Ok(Effect::ShowResult(_))
    ));
    assert!(matches!(
        core.result_action(RESULT_ID, ResultAction::Copy, 2),
        Ok(Effect::Copy(_))
    ));
    assert!(matches!(
        core.result_action(RESULT_ID, ResultAction::Open, 2),
        Ok(Effect::Open(_))
    ));
    let expected = format!("https://uploadimg.vercel.app/i/{RESULT_ID}");
    assert!(Core::valid_result(RESULT_ID, &expected));
    for url in [
        expected.replace("https:", "http:"),
        expected.replace("uploadimg.vercel.app", "evil.invalid"),
        format!("{expected}?query=1"),
        format!("{expected}#fragment"),
        expected.replace("uploadimg.vercel.app", "u@uploadimg.vercel.app"),
        expected.replace("uploadimg.vercel.app", "uploadimg.vercel.app:443"),
    ] {
        assert!(!Core::valid_result(RESULT_ID, &url));
    }
}
#[test]
fn result_history_is_bounded_and_expires_at_24_hours() {
    let mut core = Core::default();
    for n in 0..25u8 {
        let id = format!("{n:032x}");
        let result_id = format!("{n:048x}");
        core.admit(id.clone(), image(n), Source::Synthetic, n as u64, false)
            .unwrap();
        core.act(&id, CandidateAction::Share, &result_id, n as u64)
            .unwrap();
        core.record_result(
            result_id.clone(),
            format!("https://uploadimg.vercel.app/i/{result_id}"),
            n as u64,
            86_400_000,
        )
        .unwrap();
    }
    assert_eq!(core.results().len(), 20);
    assert_eq!(core.results()[0].id, format!("{:048x}", 5));
    core.expire(86_400_024);
    assert!(core.results().is_empty());
    assert!(core
        .result_action(&format!("{:048x}", 24), ResultAction::Copy, 86_400_024)
        .is_err());
}
