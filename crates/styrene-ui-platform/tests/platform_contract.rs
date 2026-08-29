use styrene_ui_platform::{
    AccessibilityPreferences, Appearance, ApplicationLifecycle, AuthorizationState, Contrast,
    EdgeInsets, KeyboardGeometry, MotionPreference, PermissionKind, PermissionStatus,
    PlatformApplyResult, PlatformChange, PlatformEvent, PlatformGeometry, PlatformInsets,
    PlatformSnapshot, PlatformState, WindowClass, WindowMetrics,
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
            text_scale_percent: 100,
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
fn current_generation_events_update_typed_platform_facts() {
    let mut state = PlatformState::new(snapshot(4, 10));

    assert_eq!(
        state.apply_event(PlatformEvent::Changed {
            generation: 4,
            sequence: 11,
            change: PlatformChange::Accessibility(AccessibilityPreferences {
                text_scale_percent: 200,
                appearance: Appearance::Dark,
                contrast: Contrast::Increased,
                motion: MotionPreference::Reduced,
            }),
        }),
        PlatformApplyResult::Applied
    );
    assert_eq!(state.snapshot().accessibility.text_scale_percent, 200);
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
