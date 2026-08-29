use styrene_ui_platform::{
    AccessibilityPreferences, AndroidUsbAttachment, Appearance, ApplicationLifecycle,
    AuthorizationState, Contrast, EdgeInsets, KeyboardGeometry, MotionPreference, PermissionKind,
    PermissionStatus, PlatformApplyResult, PlatformChange, PlatformEvent, PlatformGeometry,
    PlatformInsets, PlatformSnapshot, PlatformState, TextScale, TextScaleCategory, WindowClass,
    WindowMetrics,
};

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
