use super::*;
use crate::events::{Event, ImageEvent};
fn image(generation: u64, source: i32) -> Event {
    let mut thumbnail = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    thumbnail.extend(2u32.to_be_bytes());
    thumbnail.extend(2u32.to_be_bytes());
    Event::Image(ImageEvent {
        generation,
        lifecycle: 0,
        source,
        width: 2,
        height: 2,
        digest: [7; 32],
        original: b"original-never-crosses-ipc".to_vec(),
        thumbnail,
    })
}
fn shows_window(operations: &[Operation]) -> bool {
    operations
        .iter()
        .any(|op| matches!(op, Operation::ShowWindow { .. }))
}
#[test]
fn admitted_automatic_clipboard_selects_prepared_preview_and_shows_without_sharing() {
    let mut model = Model::new(true);
    model.set_clipboard(true, 0).unwrap();
    let operations = model.apply_event(image(model.generation, 1), 1);
    assert!(
        shows_window(&operations),
        "admitted automatic image must foreground"
    );
    let state = model.snapshot(1);
    assert_eq!(state.pending.len(), 1);
    assert_eq!(
        state.selected_candidate_id.as_deref(),
        Some(state.pending[0].id.as_str())
    );
    assert!(state.pending[0]
        .preview_data_url
        .starts_with("data:image/png;base64,"));
    assert!(state.results.is_empty());
    assert!(operations
        .iter()
        .any(|op| matches!(op, Operation::Notify { result: false, .. })));
}
#[test]
fn admitted_explicit_paste_while_off_selects_preview_and_shows_without_opt_in_or_sharing() {
    let mut model = Model::new(true);
    let operations = model.apply_event(image(0, 2), 0);
    assert!(
        shows_window(&operations),
        "admitted explicit Paste must foreground"
    );
    let state = model.snapshot(0);
    assert!(!state.clipboard_enabled);
    assert_eq!(state.pending.len(), 1);
    assert_eq!(
        state.selected_candidate_id.as_deref(),
        Some(state.pending[0].id.as_str())
    );
    assert!(state.pending[0]
        .preview_data_url
        .starts_with("data:image/png;base64,"));
    assert!(state.results.is_empty());
}
#[test]
fn rejected_stale_duplicate_paused_and_full_clipboard_events_never_foreground() {
    let mut off = Model::new(true);
    assert!(!shows_window(&off.apply_event(image(0, 1), 0)));
    let mut model = Model::new(true);
    model.set_clipboard(true, 0).unwrap();
    assert!(!shows_window(&model.apply_event(image(0, 1), 1)));
    let Event::Image(mut stale) = image(model.generation, 2) else {
        unreachable!()
    };
    stale.lifecycle = 1;
    assert!(!shows_window(&model.apply_event(Event::Image(stale), 1)));
    let Event::Image(mut malformed) = image(model.generation, 2) else {
        unreachable!()
    };
    malformed.thumbnail.clear();
    assert!(!shows_window(
        &model.apply_event(Event::Image(malformed), 1)
    ));
    model.apply_event(image(model.generation, 1), 2);
    assert!(!shows_window(
        &model.apply_event(image(model.generation, 2), 3)
    ));
    for digest in [8, 9] {
        let Event::Image(mut fresh) = image(model.generation, 2) else {
            unreachable!()
        };
        fresh.digest = [digest; 32];
        model.apply_event(Event::Image(fresh), 4);
    }
    let Event::Image(mut overflow) = image(model.generation, 2) else {
        unreachable!()
    };
    overflow.digest = [10; 32];
    assert!(!shows_window(&model.apply_event(Event::Image(overflow), 5)));
    assert_eq!(model.core.pending().len(), 3);
    model.set_paused(true, 6);
    assert!(!shows_window(
        &model.apply_event(image(model.generation, 1), 7)
    ));
    assert!(!shows_window(
        &model.apply_event(image(model.generation, 2), 7)
    ));
    assert!(model.core.pending().is_empty());
    assert!(model.core.results().is_empty());
}
#[test]
fn synthetic_selection_keeps_existing_no_foreground_behavior_and_tray_show_remains_explicit() {
    let mut model = Model::new(true);
    assert!(!shows_window(&model.apply_event(image(0, 0), 0)));
    assert_eq!(model.core.pending().len(), 1);
    assert!(shows_window(&model.apply_event(
        Event::Control {
            kind: 6,
            generation: 0,
            lifecycle: 0,
            code: 7,
            text: String::new(),
        },
        1
    )));
    assert!(model.core.results().is_empty());
}
#[test]
fn delayed_clipboard_foreground_requires_the_same_live_selected_prepared_candidate() {
    let mut model = Model::new(true);
    let operations = model.apply_event(image(0, 2), 0);
    let id = model.core.pending()[0].id.clone();
    assert!(operations.iter().any(|op| matches!(op, Operation::ShowWindow { candidate_id: Some(candidate) } if candidate == &id)));
    assert!(model.can_foreground_candidate(&id, 1));
    let Event::Image(mut fresh) = image(0, 2) else {
        unreachable!()
    };
    fresh.digest = [8; 32];
    model.apply_event(Event::Image(fresh), 2);
    let new_id = model.core.pending()[1].id.clone();
    assert!(!model.can_foreground_candidate(&id, 2));
    assert!(model.can_foreground_candidate(&new_id, 2));
    model.set_paused(true, 3);
    assert!(!model.can_foreground_candidate(&new_id, 3));
}
#[test]
fn delayed_clipboard_foreground_is_cancelled_after_off_ignore_share_wake_or_expiry() {
    for cancellation in ["off", "ignore", "share", "wake", "expiry"] {
        let mut model = Model::new(true);
        model.apply_event(image(0, 2), 0);
        let id = model.core.pending()[0].id.clone();
        match cancellation {
            "off" => {
                model.set_clipboard(false, 1).unwrap();
            }
            "ignore" | "share" => {
                model.candidate_action(&id, cancellation, 1).unwrap();
            }
            "wake" => {
                model.apply_event(
                    Event::Control {
                        kind: 5,
                        generation: 0,
                        lifecycle: 1,
                        code: 0,
                        text: String::new(),
                    },
                    1,
                );
            }
            "expiry" => {}
            _ => unreachable!(),
        }
        let now = if cancellation == "expiry" { 600_000 } else { 1 };
        assert!(
            !model.can_foreground_candidate(&id, now),
            "delayed foreground survived {cancellation}"
        );
    }
}
#[test]
fn ipc_snapshot_exposes_only_bounded_preview_and_metadata_never_original_bytes() {
    let mut model = Model::new(true);
    model.apply_event(image(0, 0), 0);
    let state = serde_json::to_value(model.snapshot(1)).unwrap();
    assert_eq!(state["simulation"], true);
    assert_eq!(state["folderAvailable"], false);
    assert_eq!(state["clipboardEnabled"], false);
    assert_eq!(state["notificationStatus"], "not_requested");
    assert_eq!(state["pending"][0]["expiresInSeconds"], 600);
    assert_eq!(
        state["pending"][0]["inputBytes"],
        b"original-never-crosses-ipc".len()
    );
    assert!(state["pending"][0]["previewDataUrl"]
        .as_str()
        .unwrap()
        .starts_with("data:image/png;base64,"));
    assert!(!serde_json::to_string(&state)
        .unwrap()
        .contains("original-never-crosses-ipc"));
    assert_eq!(state["pending"][0].as_object().unwrap().len(), 7);
    assert!(state["results"].as_array().unwrap().is_empty());
}
#[test]
fn generation_fence_denial_and_wake_preserve_disabled_paused_source_semantics() {
    let mut model = Model::new(true);
    model.set_clipboard(true, 0).unwrap();
    model.apply_event(image(0, 1), 1);
    assert!(model.core.pending().is_empty());
    model.apply_event(image(1, 1), 2);
    assert_eq!(model.core.pending().len(), 1);
    model.apply_event(
        Event::Control {
            kind: 3,
            generation: 1,
            lifecycle: 0,
            code: 0,
            text: String::new(),
        },
        3,
    );
    assert!(!model.core.clipboard_enabled());
    assert!(model.core.pending().is_empty());
    assert!(model.previews.is_empty());
    model.set_paused(true, 4);
    model.apply_event(
        Event::Control {
            kind: 5,
            generation: model.generation,
            lifecycle: 1,
            code: 0,
            text: String::new(),
        },
        5,
    );
    assert!(model.core.paused());
    assert!(!model.core.clipboard_enabled());
}
#[test]
fn os_notification_action_is_processed_without_get_state_or_frontend_poll() {
    let mut model = Model::new(true);
    model.apply_event(image(0, 0), 0);
    let id = model.core.pending()[0].id.clone();
    model.apply_event(
        Event::Control {
            kind: 6,
            generation: 0,
            lifecycle: 0,
            code: 1,
            text: id.clone(),
        },
        1,
    );
    assert!(model.core.results().is_empty());
    assert_eq!(model.core.pending().len(), 1);
    let operations = model.apply_event(
        Event::Control {
            kind: 6,
            generation: 0,
            lifecycle: 0,
            code: 2,
            text: id.clone(),
        },
        2,
    );
    assert_eq!(model.core.results().len(), 1);
    assert!(model.previews.is_empty());
    assert!(operations
        .iter()
        .any(|op| matches!(op, Operation::Notify { result: true, .. })));
    model.apply_event(
        Event::Control {
            kind: 6,
            generation: 0,
            lifecycle: 0,
            code: 2,
            text: id,
        },
        3,
    );
    assert_eq!(model.core.results().len(), 1);
    assert!(model.result_action("../../path", "copy", 3).is_err());
    let result = model.core.results()[0].id.clone();
    assert!(model
        .result_action(&result, "open arbitrary-url", 3)
        .is_err());
    model.tick(86_400_002);
    assert!(model.result_action(&result, "copy", 86_400_002).is_err());
}
#[test]
fn malformed_native_thumbnail_is_rejected_before_ui_state_exposure() {
    let mut model = Model::new(true);
    let Event::Image(mut event) = image(0, 0) else {
        unreachable!()
    };
    event.thumbnail = b"not-a-prepared-thumbnail".to_vec();
    model.apply_event(Event::Image(event), 0);
    assert!(model.core.pending().is_empty());
    assert!(model.previews.is_empty());
}
#[test]
fn unavailable_platform_does_not_fabricate_native_images_or_permission_success() {
    let mut model = Model::new(false);
    assert!(model.set_clipboard(true, 0).is_err());
    assert!(model.request_notifications().is_err());
    model.apply_event(image(0, 0), 0);
    assert!(model.core.pending().is_empty());
    let state = serde_json::to_value(model.snapshot(0)).unwrap();
    assert_eq!(state["notificationStatus"], "unavailable");
    assert_eq!(state["nativeAvailable"], false);
}

#[test]
fn effect_batches_capture_monotonic_order_and_post_share_identity_without_source_change() {
    let mut model = Model::new(true);
    let operations = model.apply_event(image(0, 0), 0);
    let before = model.effect_batch(operations);
    assert_eq!(before.live_ids.len(), 1);
    let candidate = model.core.pending()[0].id.clone();
    let generation = model.generation;
    let operations = model.candidate_action(&candidate, "share", 1).unwrap();
    let shared = model.effect_batch(operations);
    assert!(shared.revision > before.revision);
    assert_eq!(model.generation, generation);
    assert_eq!(
        shared.live_ids,
        vec![format!("result.{}", model.core.results()[0].id)]
    );
    assert!(!shared.live_ids.contains(&before.live_ids[0]));
    model.tick(86_400_001);
    let expired = model.effect_batch(vec![]);
    assert!(expired.revision > shared.revision);
    assert!(expired.live_ids.is_empty());
}

#[test]
fn one_worker_batch_applies_only_final_source_control_without_transient_enable() {
    let mut model = Model::new(true);
    let mut operations = model.set_clipboard(true, 0).unwrap();
    operations.extend(model.set_paused(true, 1));
    let batch = model.effect_batch(operations);
    let controls: Vec<_> = batch
        .operations
        .iter()
        .filter_map(|op| match op {
            Operation::Configure {
                enabled,
                paused,
                generation,
                ..
            } => Some((*enabled, *paused, *generation)),
            _ => None,
        })
        .collect();
    assert_eq!(controls, [(true, true, model.generation)]);
}

#[test]
fn queued_wake_and_explicit_paste_before_off_cannot_reappear_but_new_paste_can() {
    let mut model = Model::new(true);
    model.set_clipboard(true, 0).unwrap();
    // Native wake and explicit paste are already queued, before Rust sees OFF.
    // The former single G2 becomes (Rust G1, native lifecycle 2).
    let wake = Event::Control {
        kind: 5,
        generation: 1,
        lifecycle: 2,
        code: 0,
        text: String::new(),
    };
    let Event::Image(mut old_paste) = image(2, 2) else {
        unreachable!()
    };
    old_paste.generation = 1;
    old_paste.lifecycle = 2;
    model.set_clipboard(false, 1).unwrap();
    model.apply_event(wake, 2);
    model.apply_event(Event::Image(old_paste), 2);
    assert!(
        model.core.pending().is_empty(),
        "pre-OFF paste reappeared after queued wake"
    );
    assert!(model.core.results().is_empty());
    // The synchronous OFF control reports its new native lifecycle before any
    // new paste callback; an old in-flight callback can arrive after that report.
    model.apply_event(
        Event::Control {
            kind: 5,
            generation: model.generation,
            lifecycle: 3,
            code: 1,
            text: String::new(),
        },
        3,
    );
    let Event::Image(mut in_flight) = image(1, 2) else {
        unreachable!()
    };
    in_flight.lifecycle = 2;
    model.apply_event(Event::Image(in_flight), 3);
    assert!(model.core.pending().is_empty());
    let Event::Image(mut fresh) = image(model.generation, 2) else {
        unreachable!()
    };
    fresh.lifecycle = 3;
    model.apply_event(Event::Image(fresh), 4);
    assert_eq!(
        model.core.pending().len(),
        1,
        "new explicit paste while OFF must remain allowed"
    );
    assert!(!model.core.clipboard_enabled());
}

#[test]
fn wake_invalidates_queued_images_even_when_rust_control_token_is_unchanged() {
    let mut model = Model::new(true);
    model.apply_event(
        Event::Control {
            kind: 5,
            generation: 0,
            lifecycle: 1,
            code: 0,
            text: String::new(),
        },
        0,
    );
    model.apply_event(image(0, 2), 1);
    assert!(model.core.pending().is_empty());
    let Event::Image(mut fresh) = image(0, 2) else {
        unreachable!()
    };
    fresh.lifecycle = 1;
    model.apply_event(Event::Image(fresh), 2);
    assert_eq!(model.core.pending().len(), 1);
    assert_eq!(model.generation, 0);
}

fn lifecycle_event(generation: u64, lifecycle: u64, code: i32) -> Event {
    Event::Control {
        kind: 5,
        generation,
        lifecycle,
        code,
        text: String::new(),
    }
}

#[test]
fn clipboard_on_ack_preserves_existing_synthetic_candidate_preview_and_selection() {
    let mut model = Model::new(true);
    model.apply_event(image(0, 0), 0);
    let id = model.core.pending()[0].id.clone();
    let preview = model.previews[&id].clone();
    model.set_clipboard(true, 1).unwrap();
    let operations = model.apply_event(lifecycle_event(1, 1, 1), 2);
    assert_eq!(
        model.core.pending().len(),
        1,
        "ON acknowledgment purged synthetic"
    );
    assert_eq!(model.core.pending()[0].id, id);
    assert_eq!(model.previews[&id], preview);
    assert_eq!(model.selected_candidate.as_ref(), Some(&id));
    assert!(model.core.clipboard_enabled());
    assert!(!operations
        .iter()
        .any(|op| matches!(op, Operation::Configure { .. })));
    assert_eq!(model.lifecycle, 1);
}

#[test]
fn clipboard_off_ack_removes_only_clipboard_candidates_and_keeps_synthetic() {
    let mut model = Model::new(true);
    model.set_clipboard(true, 0).unwrap();
    model.apply_event(lifecycle_event(1, 1, 1), 1);
    for source in [0, 1] {
        let Event::Image(mut event) = image(1, source) else {
            unreachable!()
        };
        event.lifecycle = 1;
        event.digest = [source as u8; 32];
        model.apply_event(Event::Image(event), 2);
    }
    assert_eq!(model.core.pending().len(), 2);
    let synthetic = model.core.pending()[0].id.clone();
    model.set_clipboard(false, 3).unwrap();
    assert_eq!(
        model.core.pending().len(),
        1,
        "OFF must remove clipboard immediately"
    );
    model.apply_event(lifecycle_event(2, 2, 1), 4);
    assert_eq!(
        model.core.pending().len(),
        1,
        "OFF acknowledgment purged synthetic"
    );
    assert_eq!(model.core.pending()[0].id, synthetic);
    assert_eq!(model.previews.len(), 1);
    assert!(model.previews.contains_key(&synthetic));
    assert!(!model.core.clipboard_enabled());
    assert!(!model.core.paused());
}

#[test]
fn real_wake_purges_pending_across_interleaved_control_and_acknowledgment() {
    // Native callbacks are FIFO. Rust may change control while an earlier ACK
    // or wake waits in that queue, so wake can carry an obsolete control token.
    for events in [
        vec![(1, 2, 0), (2, 3, 1)],            // wake before native OFF
        vec![(1, 2, 1), (1, 3, 0), (2, 4, 1)], // old ACK, wake, OFF ACK
        vec![(2, 2, 1), (2, 3, 0)],            // OFF ACK before wake
    ] {
        let mut model = Model::new(true);
        model.set_clipboard(true, 0).unwrap();
        model.apply_event(lifecycle_event(1, 1, 1), 1);
        let Event::Image(mut synthetic) = image(1, 0) else {
            unreachable!()
        };
        synthetic.lifecycle = 1;
        model.apply_event(Event::Image(synthetic), 2);
        assert_eq!(model.core.pending().len(), 1);
        model.set_clipboard(false, 3).unwrap();
        for (generation, lifecycle, code) in events {
            let operations = model.apply_event(lifecycle_event(generation, lifecycle, code), 4);
            if code == 0 {
                assert!(
                    model.core.pending().is_empty(),
                    "wake skipped after Rust OFF"
                );
                assert!(model.previews.is_empty());
                assert!(model.selected_candidate.is_none());
                assert!(!operations
                    .iter()
                    .any(|op| matches!(op, Operation::Configure { .. })));
            }
        }
        assert!(!model.core.clipboard_enabled());
        assert!(!model.core.paused());
        assert_eq!(model.generation, 2);
    }
}
