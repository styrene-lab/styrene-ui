use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use styrene_ui_platform::{
    AccessibilityPreferences, AndroidUsbAttachment, Appearance, ApplicationLifecycle,
    AuthorizationState, ClipboardTextReader, ClipboardTextWriter, Contrast, EdgeInsets,
    KeyboardGeometry, MAX_CANDIDATE_PAYLOAD_BYTES, MockClipboardTextReader,
    MockClipboardTextWriter, MockQrDestinationScanner, MockTextAcquisitionResponse,
    MotionPreference, PermissionKind, PermissionStatus, PlatformApplyResult, PlatformChange,
    PlatformEvent, PlatformFailure, PlatformGeometry, PlatformInsets, PlatformSnapshot,
    PlatformState, QrDestinationScanner, TextAcquisitionCompletion, TextAcquisitionFailure,
    TextAcquisitionGeneration, TextScale, TextScaleCategory, WindowClass, WindowMetrics,
};

fn ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = pin!(future);
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("deterministic platform mock unexpectedly returned pending"),
    }
}

fn snapshot(generation: u64, sequence: u64) -> PlatformSnapshot {
    PlatformSnapshot {
        generation,
        sequence,
        window: WindowMetrics {
            class: WindowClass::Compact,
            width_css_px: 390,
            height_css_px: 844,
        },
        accessibility: AccessibilityPreferences {
            text_scale: TextScale::Percent(100),
            appearance: Appearance::Light,
            contrast: Contrast::Standard,
            motion: MotionPreference::Full,
        },
        geometry: PlatformGeometry {
            insets: PlatformInsets::CssEnvironment,
            keyboard: KeyboardGeometry::WebViewManaged { visible: false },
        },
        lifecycle: ApplicationLifecycle::Active,
        permissions: Vec::new(),
        notification_authorization: AuthorizationState::NotDetermined,
    }
}

#[test]
fn text_scale_categories_have_stable_platform_attributes() {
    assert_eq!(TextScaleCategory::Large.as_str(), "large");
    assert_eq!(
        TextScaleCategory::AccessibilityExtraExtraExtraLarge.as_str(),
        "accessibility-extra-extra-extra-large"
    );
    assert_eq!(TextScaleCategory::Unknown.as_str(), "unknown");
}

#[test]
fn android_usb_identity_is_attachment_scoped() {
    let attachment = AndroidUsbAttachment {
        device_id: 7,
        vendor_id: 0x10c4,
        product_id: 0xea60,
        device_name: "/dev/bus/usb/001/007".into(),
    };

    assert_eq!(attachment.device_id, 7);
    assert_eq!(attachment.vendor_id, 0x10c4);
    assert_eq!(attachment.product_id, 0xea60);
    assert_eq!(attachment.device_name, "/dev/bus/usb/001/007");
}

#[test]
fn current_generation_events_update_typed_platform_facts() {
    let mut state = PlatformState::new(snapshot(4, 10));

    assert_eq!(
        state.apply_event(PlatformEvent::Changed {
            generation: 4,
            sequence: 11,
            change: PlatformChange::Accessibility(AccessibilityPreferences {
                text_scale: TextScale::Percent(200),
                appearance: Appearance::Dark,
                contrast: Contrast::Increased,
                motion: MotionPreference::Reduced,
            }),
        }),
        PlatformApplyResult::Applied
    );
    assert_eq!(state.snapshot().accessibility.text_scale, TextScale::Percent(200));
    assert_eq!(state.snapshot().accessibility.contrast, Contrast::Increased);
    assert_eq!(state.snapshot().accessibility.motion, MotionPreference::Reduced);

    assert_eq!(
        state.apply_event(PlatformEvent::Changed {
            generation: 4,
            sequence: 12,
            change: PlatformChange::Permission(PermissionStatus {
                kind: PermissionKind::Bluetooth,
                state: AuthorizationState::Denied,
            }),
        }),
        PlatformApplyResult::Applied
    );
    assert_eq!(state.snapshot().permissions.len(), 1);
    assert_eq!(state.snapshot().permissions[0].state, AuthorizationState::Denied);
}

#[test]
fn stale_callbacks_cannot_replace_current_platform_state() {
    let mut state = PlatformState::new(snapshot(8, 20));

    assert_eq!(
        state.apply_event(PlatformEvent::Changed {
            generation: 7,
            sequence: 99,
            change: PlatformChange::Lifecycle(ApplicationLifecycle::Background),
        }),
        PlatformApplyResult::IgnoredStale
    );
    assert_eq!(
        state.apply_event(PlatformEvent::Changed {
            generation: 8,
            sequence: 20,
            change: PlatformChange::Window(WindowMetrics {
                class: WindowClass::Wide,
                width_css_px: 1024,
                height_css_px: 768,
            }),
        }),
        PlatformApplyResult::IgnoredStale
    );
    assert_eq!(state.snapshot(), &snapshot(8, 20));
}

#[test]
fn bounded_stream_lag_requires_an_authoritative_resnapshot() {
    let mut state = PlatformState::new(snapshot(3, 7));

    assert_eq!(
        state.apply_event(PlatformEvent::ResyncRequired { generation: 3, dropped_events: 2 }),
        PlatformApplyResult::ResyncRequired
    );
    assert_eq!(
        state.apply_event(PlatformEvent::ResyncRequired { generation: 2, dropped_events: 2 }),
        PlatformApplyResult::IgnoredStale
    );
}

#[test]
fn newer_snapshot_changes_generation_without_merging_partial_state() {
    let mut state = PlatformState::new(snapshot(1, 5));
    let mut replacement = snapshot(2, 1);
    replacement.geometry = PlatformGeometry {
        insets: PlatformInsets::NativeBridge(EdgeInsets {
            top_css_px: 24,
            right_css_px: 0,
            bottom_css_px: 16,
            left_css_px: 0,
        }),
        keyboard: KeyboardGeometry::NativeBridge { occluded_height_css_px: 320 },
    };

    assert_eq!(state.replace_snapshot(replacement.clone()), PlatformApplyResult::Applied);
    assert_eq!(state.snapshot(), &replacement);
    assert_eq!(state.replace_snapshot(snapshot(1, 100)), PlatformApplyResult::IgnoredStale);
}

#[test]
fn authoritative_resync_accepts_native_changes_at_equal_web_sequence() {
    let mut state = PlatformState::new(snapshot(4, 9));
    let mut native_update = snapshot(4, 9);
    native_update.accessibility.text_scale = TextScale::Percent(135);

    assert_eq!(
        state.replace_resynced_snapshot(native_update.clone()),
        PlatformApplyResult::Applied
    );
    assert_eq!(state.snapshot(), &native_update);
    assert_eq!(state.replace_resynced_snapshot(snapshot(4, 8)), PlatformApplyResult::IgnoredStale);
}

#[test]
fn clipboard_mock_types_boundary_failures_and_preserves_generation() {
    let oversized = vec![b'x'; MAX_CANDIDATE_PAYLOAD_BYTES + 1];
    let reader = MockClipboardTextReader::new([
        MockTextAcquisitionResponse::Denied,
        MockTextAcquisitionResponse::Restricted,
        MockTextAcquisitionResponse::Unavailable,
        MockTextAcquisitionResponse::Cancelled,
        MockTextAcquisitionResponse::ServiceBytes(oversized),
        MockTextAcquisitionResponse::ServiceBytes(vec![0xff]),
        MockTextAcquisitionResponse::ServiceBytes(b"not-validated-by-platform".to_vec()),
    ]);

    let expected = [
        Err(TextAcquisitionFailure::Denied),
        Err(TextAcquisitionFailure::Restricted),
        Err(TextAcquisitionFailure::Unavailable),
        Err(TextAcquisitionFailure::Cancelled),
        Err(TextAcquisitionFailure::Oversized),
        Err(TextAcquisitionFailure::Malformed),
    ];
    for (value, expected) in expected.into_iter().enumerate() {
        let generation = TextAcquisitionGeneration::new(value as u64 + 10);
        let completion = ready(reader.read_clipboard_text(generation));
        assert_eq!(completion.generation, generation);
        assert_eq!(completion.result, expected);
    }

    let completion = ready(reader.read_clipboard_text(TextAcquisitionGeneration::new(16)));
    assert_eq!(completion.result.unwrap().as_str(), "not-validated-by-platform");
}

#[test]
fn clipboard_writer_preserves_the_exact_public_value_and_typed_failure() {
    let writer = MockClipboardTextWriter::new([
        Ok(()),
        Err(PlatformFailure { code: "clipboard_denied".into(), retryable: false }),
    ]);
    let public_destination = "e01b09b22ccc4e2755d29eead962677b";

    assert_eq!(ready(writer.write_clipboard_text(public_destination.into())), Ok(()));
    assert_eq!(
        ready(writer.write_clipboard_text(public_destination.into())),
        Err(PlatformFailure { code: "clipboard_denied".into(), retryable: false })
    );
    assert_eq!(writer.writes(), [public_destination, public_destination]);
}

#[test]
fn qr_mock_reports_cancellation_and_success_without_destination_validation() {
    let oversized = vec![b'x'; MAX_CANDIDATE_PAYLOAD_BYTES + 1];
    let scanner = MockQrDestinationScanner::new([
        MockTextAcquisitionResponse::Denied,
        MockTextAcquisitionResponse::Restricted,
        MockTextAcquisitionResponse::Unavailable,
        MockTextAcquisitionResponse::ServiceBytes(oversized),
        MockTextAcquisitionResponse::ServiceBytes(vec![0xff]),
        MockTextAcquisitionResponse::Cancelled,
        MockTextAcquisitionResponse::ServiceBytes(Vec::new()),
    ]);

    let expected = [
        TextAcquisitionFailure::Denied,
        TextAcquisitionFailure::Restricted,
        TextAcquisitionFailure::Unavailable,
        TextAcquisitionFailure::Oversized,
        TextAcquisitionFailure::Malformed,
    ];
    for (value, expected) in expected.into_iter().enumerate() {
        let completion =
            ready(scanner.scan_qr_destination(TextAcquisitionGeneration::new(value as u64 + 21)));
        assert_eq!(completion.result, Err(expected));
    }

    let cancelled = ready(scanner.scan_qr_destination(TextAcquisitionGeneration::new(26)));
    assert_eq!(cancelled.generation.value(), 26);
    assert_eq!(cancelled.result, Err(TextAcquisitionFailure::Cancelled));

    let candidate = ready(scanner.scan_qr_destination(TextAcquisitionGeneration::new(27)));
    assert_eq!(candidate.result.unwrap().as_str(), "");

    let exhausted = ready(scanner.scan_qr_destination(TextAcquisitionGeneration::new(28)));
    assert_eq!(exhausted.result, Err(TextAcquisitionFailure::Unavailable));
}

#[test]
fn text_acquisition_completion_rejects_stale_generation() {
    let current = TextAcquisitionGeneration::new(12);
    let stale = TextAcquisitionCompletion {
        generation: TextAcquisitionGeneration::new(11),
        result: Err(TextAcquisitionFailure::Denied),
    };
    let matching = TextAcquisitionCompletion {
        generation: current,
        result: Err(TextAcquisitionFailure::Cancelled),
    };

    assert_eq!(stale.into_result_for(current), None);
    assert_eq!(matching.into_result_for(current), Some(Err(TextAcquisitionFailure::Cancelled)));
}

#[test]
fn candidate_payload_bound_is_utf8_bytes_and_accepts_the_exact_limit() {
    let exact = "x".repeat(MAX_CANDIDATE_PAYLOAD_BYTES);
    let multibyte = "\u{1f642}".repeat(MAX_CANDIDATE_PAYLOAD_BYTES / 4 + 1);

    assert_eq!(
        styrene_ui_platform::CandidatePayload::new(exact.clone())
            .expect("exact byte boundary")
            .into_string(),
        exact
    );
    assert_eq!(
        styrene_ui_platform::CandidatePayload::new(multibyte),
        Err(styrene_ui_platform::CandidatePayloadError::Oversized)
    );
}
