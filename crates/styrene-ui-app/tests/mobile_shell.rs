use dioxus::prelude::*;
use styrene_ui_app::{
    BackNavigation, Composer, IdentityBootstrap, IdentityQrCode, IdentityRecoveryPanel,
    LocalAnnounceStatus, MobileShell, NewMessageForm, PropagationPanel,
};
use styrene_ui_platform::{
    AccessibilityPreferences, AndroidUsbAttachment, AppLockPolicy, Appearance,
    ApplicationLifecycle, AuthorizationState, BleAdapterState, BleApprovedPeripheral, BleCandidate,
    BleControlFailure, BleControlPhase, BleControlState, BlePeripheralId, Contrast,
    KeyboardGeometry, MotionPreference, PermissionKind, PermissionStatus, PlatformGeometry,
    PlatformInsets, PlatformSnapshot, TextScale, TextScaleCategory, WindowClass, WindowMetrics,
};
use styrene_ui_state::{
    ApplyResult, BearerState, Conversation, IdentityCustody, IdentityCustodyAuthentication,
    IdentityCustodyAvailability, IdentityCustodyBackend, IdentityCustodyDowngrade,
    IdentityCustodyProtection, IdentityRecoveryFailure, IdentityRecoveryPhase,
    IdentityRecoveryState, LocalAnnounceOutcome, MobileFixture, MobileMinimumCorpus, MobileStore,
    PropagationCandidate, PropagationPolicy, PropagationProgress, PropagationSynchronization,
    PropagationTerminalOutcome, PropagationTriggerSource, PropagationUpdate, RuntimeBoundary,
    SessionPhase, SyncState, TargetClass, TypedFailure,
};

const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");
const MOBILE_CSS: &str = include_str!("../assets/mobile.css");

fn fixture(id: &str) -> MobileFixture {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("mobile minimum fixture must deserialize");
    corpus
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == id)
        .unwrap_or_else(|| panic!("missing fixture {id}"))
}

fn render(fixture: MobileFixture) -> String {
    dioxus_ssr::render_element(rsx! {
        MobileShell { target: TargetClass::Ios, fixture }
    })
}

#[test]
fn ios_more_exposes_app_lock_without_conflating_identity_custody() {
    let markup = dioxus_ssr::render_element(rsx! {
        MobileShell {
            target: TargetClass::Ios,
            fixture: fixture("live-empty-connected"),
            app_lock_policy: AppLockPolicy::EveryLaunch,
        }
    });

    assert!(markup.contains("id=\"mobile.app-lock\""));
    assert!(markup.contains("Every app launch"));
    assert!(markup.contains("Once after device reboot"));
    assert!(markup.contains("Identity custody remains protected separately"));
}

#[component]
fn AppLockShell(target: TargetClass, policy: AppLockPolicy, changeable: bool) -> Element {
    let fixture = fixture("live-empty-connected");
    if changeable {
        rsx! {
            MobileShell {
                target,
                fixture,
                app_lock_policy: policy,
                app_lock_policy_change: move |_policy: AppLockPolicy| {},
            }
        }
    } else {
        rsx! {
            MobileShell { target, fixture, app_lock_policy: policy }
        }
    }
}

#[component]
fn AppLockFailureShell(code: String, retry_available: bool) -> Element {
    let fixture = app_lock_failure_fixture(&code);
    if retry_available {
        rsx! {
            MobileShell { target: TargetClass::Ios, fixture, app_unlock_retry: move |()| {} }
        }
    } else {
        rsx! {
            MobileShell { target: TargetClass::Ios, fixture }
        }
    }
}

fn render_app_lock(target: TargetClass, policy: AppLockPolicy, changeable: bool) -> String {
    dioxus_ssr::render_element(rsx! {
        AppLockShell { target, policy, changeable }
    })
}

fn app_lock_failure_fixture(code: &str) -> MobileFixture {
    let mut fixture = fixture("live-empty-connected");
    fixture.session.phase = SessionPhase::Failed;
    fixture.session.failure = Some(TypedFailure { code: code.into(), retryable: true });
    fixture
}

#[test]
fn app_lock_control_is_ios_only_even_when_a_policy_is_supplied() {
    let markup = render_app_lock(TargetClass::Android, AppLockPolicy::EveryLaunch, true);
    assert!(!markup.contains("id=\"mobile.app-lock\""), "Android rendered App Lock");
    assert!(!markup.contains("Once after device reboot"), "Android advertised App Lock");

    let without_policy = dioxus_ssr::render_element(rsx! {
        MobileShell { target: TargetClass::Ios, fixture: fixture("live-empty-connected") }
    });
    assert!(!without_policy.contains("id=\"mobile.app-lock\""));
}

#[test]
fn app_lock_control_presents_every_choice_with_the_current_selection() {
    for (policy, label) in [
        (AppLockPolicy::EveryLaunch, "Every app launch"),
        (AppLockPolicy::OncePerBoot, "Once after device reboot"),
        (AppLockPolicy::Off, "Off"),
    ] {
        let markup = render_app_lock(TargetClass::Ios, policy, true);
        let select = opening_tag_with_id(&markup, "mobile.app-lock-policy");
        assert!(select.contains(&format!("value=\"{}\"", policy.as_str())), "{policy:?}");
        assert!(!select.contains("disabled"), "{policy:?}");
        for choice in [AppLockPolicy::EveryLaunch, AppLockPolicy::OncePerBoot, AppLockPolicy::Off] {
            let option = format!("value=\"{}\"", choice.as_str());
            let start = markup.find(&format!("<option {option}")).unwrap_or_else(|| {
                markup.find(&option).unwrap_or_else(|| panic!("{policy:?} missing {choice:?}"))
            });
            let tag = &markup[start..start + markup[start..].find('>').expect("option tag")];
            assert_eq!(tag.contains("selected"), choice == policy, "{policy:?} {choice:?}: {tag}");
        }
        assert!(markup.contains(label));
        assert!(markup.contains("Every app launch"));
        assert!(markup.contains("Once after device reboot"));
        assert!(markup.contains(">Off<"));
    }
}

#[test]
fn app_lock_control_is_labelled_and_explains_custody_separation() {
    let markup = render_app_lock(TargetClass::Ios, AppLockPolicy::OncePerBoot, true);

    assert!(markup.contains("for=\"mobile.app-lock-policy\""));
    assert!(markup.contains("Require Face ID or device passcode"));
    let select = opening_tag_with_id(&markup, "mobile.app-lock-policy");
    assert!(select.contains("aria-describedby=\"mobile.app-lock-custody\""));
    assert!(markup.contains("id=\"mobile.app-lock-custody\""));
    assert!(markup.contains("Identity custody remains protected separately"));
    assert!(!markup.contains("id=\"mobile.app-lock-disabled\""));
}

#[test]
fn app_lock_control_without_a_change_handler_is_disabled_with_guidance() {
    let markup = render_app_lock(TargetClass::Ios, AppLockPolicy::EveryLaunch, false);

    let select = opening_tag_with_id(&markup, "mobile.app-lock-policy");
    assert!(select.contains("disabled"));
    assert!(
        select.contains("aria-describedby=\"mobile.app-lock-custody mobile.app-lock-disabled\"")
    );
    assert!(markup.contains("id=\"mobile.app-lock-disabled\""));
    assert!(markup.contains("App Lock policy cannot be changed in this view."));
    assert!(markup.contains("Once after device reboot"), "disabled control hid the choices");
}

#[test]
fn app_lock_failure_offers_an_explicit_retry_without_touching_custody() {
    for code in ["app_unlock_cancelled", "app_unlock_unavailable", "app_unlock_failed"] {
        let markup = dioxus_ssr::render_element(rsx! {
            AppLockFailureShell { code: code.to_owned(), retry_available: true }
        });
        let banner = opening_tag_with_id(&markup, "mobile.session-failure");
        assert!(banner.contains(&format!("data-code=\"{code}\"")));
        assert!(banner.contains("data-retryable=\"true\""));
        let retry = opening_tag_with_id(&markup, "mobile.app-unlock-retry");
        assert!(!retry.contains("disabled"), "{code}");
        assert!(retry.contains("aria-describedby=\"mobile.session-failure\""));
        assert!(markup.contains("Retry unlock"));
        assert!(markup.contains("Identity custody was not changed."));
        assert!(!markup.contains("Open Network to review connection settings"), "{code}");
    }

    let without_handler = dioxus_ssr::render_element(rsx! {
        AppLockFailureShell { code: "app_unlock_cancelled".to_owned(), retry_available: false }
    });
    assert!(opening_tag_with_id(&without_handler, "mobile.app-unlock-retry").contains("disabled"));

    let unrelated = dioxus_ssr::render_element(rsx! {
        AppLockFailureShell { code: "embedded_start_failed".to_owned(), retry_available: true }
    });
    assert!(!unrelated.contains("id=\"mobile.app-unlock-retry\""));
    assert!(unrelated.contains("Open Network to review connection settings"));
}

#[component]
fn IdentityShell(
    fixture: MobileFixture,
    #[props(default)] succeeded: bool,
    #[props(default)] failure: Option<String>,
) -> Element {
    rsx! {
        MobileShell {
            target: TargetClass::Ios,
            fixture,
            identity_copy_succeeded: succeeded,
            identity_copy_failure: failure,
            identity_copy: move |_value: String| {},
        }
    }
}

fn render_identity_actions(fixture: MobileFixture) -> String {
    dioxus_ssr::render_element(rsx! { IdentityShell { fixture } })
}

#[component]
fn RecoveryShell(state: IdentityRecoveryState) -> Element {
    rsx! {
        IdentityRecoveryPanel {
            state,
            enabled: true,
            backup: move |_protection| {},
            restore_select: move |()| {},
            restore: move |_protection| {},
        }
    }
}

fn render_recovery(state: IdentityRecoveryState) -> String {
    dioxus_ssr::render_element(rsx! { RecoveryShell { state } })
}

#[component]
fn BootstrapShell(state: IdentityRecoveryState) -> Element {
    rsx! {
        IdentityBootstrap {
            generation: 7,
            state,
            create: move |()| {},
            restore_select: move |()| {},
            restore: move |_protection| {},
        }
    }
}

fn render_bootstrap(state: IdentityRecoveryState) -> String {
    dioxus_ssr::render_element(rsx! { BootstrapShell { state } })
}

#[component]
fn BleShell(target: TargetClass, fixture: MobileFixture, state: BleControlState) -> Element {
    rsx! {
        MobileShell {
            target,
            fixture,
            ble_controls: state,
            ble_scan: move |()| {},
            ble_select: move |_id: BlePeripheralId| {},
            ble_retry: move |()| {},
            ble_cancel: move |()| {},
            ble_forget: move |()| {},
        }
    }
}

#[component]
fn ComposerShell(conversation: Conversation, propagation: PropagationUpdate) -> Element {
    rsx! {
        Composer {
            conversation: Some(conversation),
            enabled: true,
            propagation,
            action_sink: move |_action| {},
        }
    }
}

#[component]
fn ActionShell(fixture: MobileFixture) -> Element {
    rsx! {
        MobileShell {
            target: TargetClass::Ios,
            fixture,
            action_sink: move |_action| {},
        }
    }
}

#[component]
fn NewMessageShell(
    peers: Vec<styrene_ui_state::Peer>,
    generation: u64,
    initial_search: String,
    initial_destination: String,
    #[props(default)] failure: Option<TypedFailure>,
    #[props(default)] paste_failure: Option<String>,
    #[props(default)] paste_enabled: bool,
    #[props(default)] scan_failure: Option<String>,
    #[props(default)] scan_enabled: bool,
    #[props(default)] scan_busy: bool,
) -> Element {
    rsx! {
        NewMessageForm {
            peers,
            generation,
            enabled: true,
            initial_search,
            initial_destination,
            failure,
            paste_failure,
            on_paste: paste_enabled.then_some(EventHandler::new(move |()| {})),
            scan_failure,
            scan_busy,
            on_scan: scan_enabled.then_some(EventHandler::new(move |_capture| {})),
            open_application_settings: move |()| {},
            action_sink: move |_action| {},
        }
    }
}

#[component]
fn PlatformShell(
    fixture: MobileFixture,
    platform_snapshot: PlatformSnapshot,
    ble_controls: BleControlState,
) -> Element {
    rsx! {
        MobileShell {
            target: TargetClass::Android,
            fixture,
            platform_snapshot,
            ble_controls,
            open_application_settings: move |()| {},
        }
    }
}

#[test]
fn android_usb_fallback_requires_an_explicit_attachment_action() {
    let live_fixture = fixture("live-empty-connected");
    let markup = dioxus_ssr::render_element(rsx! {
        MobileShell {
            target: TargetClass::Android,
            fixture: live_fixture,
            android_usb_attachments: vec![AndroidUsbAttachment {
                device_id: 7,
                vendor_id: 0x10c4,
                product_id: 0xea60,
                device_name: "/dev/bus/usb/001/007".into(),
            }],
            android_usb_authorization: AuthorizationState::NotDetermined,
        }
    });

    assert!(markup.contains("id=\"mobile.android-usb\""));
    assert!(markup.contains("Explicitly choose an attached USB device"));
    assert!(markup.contains("10c4:ea60"));
    assert!(markup.contains("Use USB"));
    assert!(markup.contains("USB authorization: not determined"));

    let busy = dioxus_ssr::render_element(rsx! {
        MobileShell {
            target: TargetClass::Android,
            fixture: fixture("live-empty-connected"),
            android_usb_attachments: vec![AndroidUsbAttachment {
                device_id: 7,
                vendor_id: 0x10c4,
                product_id: 0xea60,
                device_name: "/dev/bus/usb/001/007".into(),
            }],
            android_usb_busy: true,
        }
    });
    assert!(busy.contains("USB request in progress"));
    assert!(busy.contains("disabled"));
}

#[test]
fn android_usb_probe_status_discloses_host_handoff_without_claiming_remote_reception() {
    let mut live_fixture = fixture("live-empty-connected");
    let usb = live_fixture
        .bearers
        .iter_mut()
        .find(|bearer| bearer.kind == styrene_ui_state::BearerKind::AndroidUsb)
        .expect("fixture must include Android USB bearer");
    usb.state = styrene_ui_state::BearerState::Connected;
    usb.reason = None;

    let markup = dioxus_ssr::render_element(rsx! {
        MobileShell {
            target: TargetClass::Android,
            fixture: live_fixture,
            android_usb_attachments: vec![AndroidUsbAttachment {
                device_id: 7,
                vendor_id: 0x10c4,
                product_id: 0xea60,
                device_name: "/dev/bus/usb/001/007".into(),
            }],
            android_usb_probe_status: "USB accepted a 172-byte KISS frame. RF and remote reception unconfirmed.",
        }
    });

    assert!(markup.contains("USB accepted a 172-byte KISS frame"));
    assert!(markup.contains("RF and remote reception unconfirmed"));
    assert!(markup.contains("id=\"mobile.android-usb-probe-status\""));
}

#[test]
fn bluetooth_controls_require_explicit_selection_and_keep_forget_reachable() {
    let approved_id = BlePeripheralId::new("approved-rnode-id").unwrap();
    let candidate_id = BlePeripheralId::new("candidate-rnode-id").unwrap();
    let markup = dioxus_ssr::render_element(rsx! {
        BleShell {
            target: TargetClass::Ios,
            fixture: fixture("live-empty-connected"),
            state: BleControlState {
                permission: AuthorizationState::Granted,
                adapter: BleAdapterState::Ready,
                phase: BleControlPhase::Reconnecting,
                candidates: vec![BleCandidate {
                    id: candidate_id,
                    display_name: Some("Field RNode".into()),
                    rssi_dbm: Some(-61),
                }],
                approved: Some(BleApprovedPeripheral { id: approved_id }),
                failure: Some(BleControlFailure::ConnectionInterrupted),
                diagnostic_code: Some("ios_ble_disconnected".into()),
            },
        }
    });

    let card = opening_tag_with_id(&markup, "mobile.bluetooth-rnode");
    assert!(card.contains("data-phase=\"reconnecting\""));
    assert!(markup.contains("Field RNode"));
    assert!(markup.contains("data-peripheral-id=\"candidate-rnode-id\""));
    assert!(markup.contains("Approve and connect Field RNode"));
    assert!(markup.contains("approved-rnode-id"));
    assert!(opening_tag_with_id(&markup, "mobile.bluetooth-scan").contains("disabled"));
    assert!(
        !markup.contains("id=\"mobile.bluetooth-retry\""),
        "no reconnect control while a connection is in flight"
    );
    assert!(!opening_tag_with_id(&markup, "mobile.bluetooth-cancel").contains("disabled"));
    assert!(!opening_tag_with_id(&markup, "mobile.bluetooth-forget").contains("disabled"));
}

#[test]
fn bluetooth_controls_render_typed_denial_and_nonretryable_failure() {
    let markup = dioxus_ssr::render_element(rsx! {
        BleShell {
            target: TargetClass::Android,
            fixture: fixture("live-empty-connected"),
            state: BleControlState {
                permission: AuthorizationState::Denied,
                adapter: BleAdapterState::Ready,
                phase: BleControlPhase::Idle,
                candidates: Vec::new(),
                approved: Some(BleApprovedPeripheral {
                    id: BlePeripheralId::new("incompatible-rnode").unwrap(),
                }),
                failure: Some(BleControlFailure::IncompatiblePeripheral),
                diagnostic_code: Some("nus_service_missing".into()),
            },
        }
    });

    assert!(markup.contains("Bluetooth permission is denied"));
    assert!(markup.contains("required Nordic UART service"));
    assert!(markup.contains("data-retryable=\"false\""));
    assert!(!markup.contains("No scan results"));
    assert!(opening_tag_with_id(&markup, "mobile.bluetooth-scan").contains("disabled"));
    assert!(opening_tag_with_id(&markup, "mobile.bluetooth-retry").contains("disabled"));
    assert!(!opening_tag_with_id(&markup, "mobile.bluetooth-forget").contains("disabled"));
}

#[test]
fn bluetooth_scanning_shows_bounded_indeterminate_progress() {
    let markup = dioxus_ssr::render_element(rsx! {
        BleShell {
            target: TargetClass::Ios,
            fixture: fixture("live-empty-connected"),
            state: BleControlState {
                permission: AuthorizationState::Granted,
                adapter: BleAdapterState::Ready,
                phase: BleControlPhase::Scanning,
                candidates: Vec::new(),
                approved: None,
                failure: None,
                diagnostic_code: None,
            },
        }
    });

    assert!(markup.contains("Scanning for RNodes"));
    assert!(markup.contains("role=\"progressbar\""));
    assert!(markup.contains("This takes up to 10 seconds"));
}

#[test]
fn bluetooth_connected_state_hides_stale_scan_and_selection_actions() {
    let approved_id = BlePeripheralId::new("connected-rnode").unwrap();
    let markup = dioxus_ssr::render_element(rsx! {
        BleShell {
            target: TargetClass::Ios,
            fixture: fixture("live-empty-connected"),
            state: BleControlState {
                permission: AuthorizationState::Granted,
                adapter: BleAdapterState::Ready,
                phase: BleControlPhase::Connected,
                candidates: vec![BleCandidate {
                    id: approved_id.clone(),
                    display_name: Some("Field RNode".into()),
                    rssi_dbm: Some(-51),
                }],
                approved: Some(BleApprovedPeripheral { id: approved_id }),
                failure: None,
                diagnostic_code: None,
            },
        }
    });

    assert!(markup.contains("The approved RNode is connected"));
    assert!(markup.contains("Disconnect and forget RNode"));
    assert!(
        opening_tag_with_id(&markup, "mobile.bluetooth-phase").contains("data-tone=\"positive\"")
    );
    assert!(!markup.contains("id=\"mobile.bluetooth-scan\""));
    assert!(!markup.contains("Use RNode"));
    assert!(!markup.contains("data-peripheral-id"));
}

#[test]
fn bluetooth_marks_the_approved_scan_result_as_non_actionable() {
    let approved_id = BlePeripheralId::new("approved-rnode").unwrap();
    let markup = dioxus_ssr::render_element(rsx! {
        BleShell {
            target: TargetClass::Ios,
            fixture: fixture("live-empty-connected"),
            state: BleControlState {
                permission: AuthorizationState::Granted,
                adapter: BleAdapterState::Ready,
                phase: BleControlPhase::Idle,
                candidates: vec![BleCandidate {
                    id: approved_id.clone(),
                    display_name: Some("Field RNode".into()),
                    rssi_dbm: Some(-61),
                }],
                approved: Some(BleApprovedPeripheral { id: approved_id }),
                failure: None,
                diagnostic_code: None,
            },
        }
    });

    let candidate = opening_tag_with_id(&markup, "mobile.bluetooth-candidate.approved-rnode");
    assert!(candidate.contains("data-disposition=\"approved\""));
    assert!(candidate.contains("disabled"));
    assert!(markup.contains("Field RNode is already approved"));
    assert!(markup.contains(">Approved</button>"));
}

fn opening_tag_with_id<'a>(markup: &'a str, id: &str) -> &'a str {
    let id = format!("id=\"{id}\"");
    let start = markup.find(&id).unwrap_or_else(|| panic!("missing {id}"));
    let end = start + markup[start..].find('>').expect("element opening tag") + 1;
    &markup[start..end]
}

#[test]
fn public_identity_actions_share_one_backend_destination() {
    let fixture = fixture("canonical-peer-discovery");
    let public_destination = fixture.session.identity_hash.clone();
    let markup = render_identity_actions(fixture);

    assert!(markup.contains(&format!("Public LXMF destination {public_destination}")));
    assert!(!opening_tag_with_id(&markup, "mobile.identity-copy").contains("disabled"));
    assert!(
        opening_tag_with_id(&markup, "mobile.identity-show-qr").contains("aria-expanded=\"false\"")
    );
    assert!(markup.contains("Copy shares only the public LXMF destination."));
    assert!(!markup.contains("private"));

    let qr =
        dioxus_ssr::render_element(rsx! { IdentityQrCode { value: public_destination.clone() } });
    assert!(
        opening_tag_with_id(&qr, "mobile.identity-qr")
            .contains(&format!("data-payload=\"{public_destination}\""))
    );
    assert!(qr.contains(&format!("QR code for public LXMF destination {public_destination}")));
    assert!(qr.contains("<rect"));
}

#[test]
fn public_identity_copy_reports_success_and_failure_without_changing_the_value() {
    let fixture = fixture("canonical-peer-discovery");
    let public_destination = fixture.session.identity_hash.clone();
    let success = dioxus_ssr::render_element(rsx! {
        IdentityShell { fixture: fixture.clone(), succeeded: true }
    });
    let failure = dioxus_ssr::render_element(rsx! {
        IdentityShell { fixture, failure: Some("clipboard_denied".into()) }
    });

    assert!(success.contains("Public destination copied."));
    assert!(success.contains(&public_destination));
    assert!(failure.contains("Public destination was not copied (clipboard_denied)."));
    assert!(failure.contains(&public_destination));
}

#[test]
fn public_identity_actions_are_unavailable_without_a_backend_destination() {
    let mut fixture = fixture("canonical-peer-discovery");
    fixture.session.identity_hash.clear();
    let markup = render_identity_actions(fixture);

    assert!(opening_tag_with_id(&markup, "mobile.identity-copy").contains("disabled"));
    assert!(opening_tag_with_id(&markup, "mobile.identity-show-qr").contains("disabled"));
    assert!(markup.contains("Public destination is not available yet."));
    assert!(!markup.contains("id=\"mobile.identity-qr\""));
}

#[test]
fn encrypted_recovery_uses_password_inputs_and_safe_presentation_state() {
    let marker = "never-render-this-passphrase";
    let markup = render_recovery(IdentityRecoveryState::default());

    assert!(markup.contains("id=\"mobile.identity-recovery\""));
    assert!(markup.contains("id=\"mobile.identity-backup-protection\""));
    assert!(markup.contains("type=\"password\""));
    assert!(markup.contains("autocomplete=\"new-password\""));
    assert!(markup.contains("Create encrypted backup"));
    assert!(markup.contains("not retained in workflow status or diagnostics"));
    assert!(markup.contains("Restore is available before this device creates an identity"));
    assert!(!markup.contains(marker));
    assert!(!markup.contains("mobile.identity-restore-form"));
}

#[test]
fn absent_identity_requires_explicit_create_or_restore_before_startup() {
    let marker = "never-render-this-artifact-or-passphrase";
    let markup = render_bootstrap(IdentityRecoveryState::default());

    assert!(markup.contains("id=\"mobile.identity-bootstrap\""));
    assert!(markup.contains("data-generation=\"7\""));
    assert!(markup.contains("before Styrene starts networking"));
    assert!(markup.contains("id=\"mobile.identity-create-confirmation\""));
    assert!(
        opening_tag_with_id(&markup, "mobile.identity-create-confirmation").contains("required")
    );
    assert!(markup.contains("id=\"mobile.identity-create\""));
    assert!(markup.contains("id=\"mobile.identity-restore-select\""));
    assert!(!markup.contains("id=\"mobile.identity-restore-form\""));
    assert!(!markup.contains(marker));
}

#[test]
fn selected_bootstrap_document_enables_only_transient_password_restore() {
    let markup = render_bootstrap(IdentityRecoveryState {
        phase: IdentityRecoveryPhase::Idle,
        failure: Some(IdentityRecoveryFailure::AuthenticationFailed),
        restore_available: true,
    });

    assert!(markup.contains("id=\"mobile.identity-restore-form\""));
    assert!(markup.contains("type=\"password\""));
    assert!(markup.contains("autocomplete=\"current-password\""));
    assert!(markup.contains("data-failure=\"authentication_failed\""));
    assert!(!markup.contains("value="));
}

#[test]
fn recovery_reports_share_presentation_without_claiming_completion() {
    let markup = render_recovery(IdentityRecoveryState {
        phase: IdentityRecoveryPhase::SharePresented,
        failure: None,
        restore_available: false,
    });

    assert!(markup.contains("ready in the system share sheet"));
    assert!(markup.contains("Saving or sharing is not yet confirmed"));
    assert!(!markup.contains("Backup saved"));
}

#[test]
fn recovery_restore_is_explicit_and_typed_only_when_preboot_allows_it() {
    let markup = render_recovery(IdentityRecoveryState {
        phase: IdentityRecoveryPhase::Idle,
        failure: Some(IdentityRecoveryFailure::AuthenticationFailed),
        restore_available: true,
    });

    assert!(markup.contains("id=\"mobile.identity-restore-select\""));
    assert!(markup.contains("id=\"mobile.identity-restore-form\""));
    assert!(markup.contains("autocomplete=\"current-password\""));
    assert!(markup.contains("data-failure=\"authentication_failed\""));
    assert!(markup.contains("could not be authenticated"));
}

#[test]
fn every_fixture_renders_the_shared_accessibility_contract_for_both_targets() {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("mobile minimum fixture must deserialize");

    for target in [TargetClass::Ios, TargetClass::Android] {
        for fixture in &corpus.fixtures {
            let markup = dioxus_ssr::render_element(rsx! {
                MobileShell { target, fixture: fixture.clone() }
            });

            assert!(
                markup.contains(&format!("data-fixture-id=\"{}\"", fixture.id)),
                "{} must identify its fixture on {target:?}",
                fixture.id
            );
            assert!(
                markup.contains(&format!("data-target=\"{}\"", target.as_str())),
                "{} must identify the target class",
                fixture.id
            );
            for accessibility_id in &corpus.required_accessibility_ids {
                assert!(
                    markup.contains(&format!("id=\"{accessibility_id}\"")),
                    "{} must render {accessibility_id} on {target:?}",
                    fixture.id
                );
            }
        }
    }
}

#[test]
fn runtime_profiles_keep_live_and_fixture_data_paths_isolated() {
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("mobile minimum fixture must deserialize");

    for fixture in &corpus.fixtures {
        let boundary = RuntimeBoundary::from(fixture.profile);
        let markup = dioxus_ssr::render_element(rsx! {
            MobileShell { target: TargetClass::Ios, fixture: fixture.clone() }
        });

        assert_eq!(boundary.live_network_allowed(), fixture.expected.live_network_enabled);
        assert_eq!(boundary.fixture_marker_visible(), fixture.expected.fixture_banner);
        assert_eq!(
            markup.contains("id=\"mobile.fixture-banner\""),
            fixture.expected.fixture_banner
        );
        assert!(markup.contains(&format!(
            "data-live-network-enabled=\"{}\"",
            fixture.expected.live_network_enabled
        )));

        if fixture.expected.live_network_enabled {
            assert!(fixture.peers.is_empty(), "Live must not substitute fixture peers");
            assert!(fixture.messages.is_empty(), "Live must not substitute fixture messages");
        } else {
            for action in ["mobile.send", "mobile.tcp-endpoint-apply", "mobile.propagation-sync"] {
                assert!(
                    opening_tag_with_id(&markup, action).contains("disabled"),
                    "fixture action {action} must be disabled"
                );
            }
        }
    }
}

#[test]
fn shared_shell_exposes_semantic_landmarks_labels_and_statuses() {
    let markup = render(fixture("direct-message-queued"));

    for required in [
        "aria-labelledby=\"mobile.app-title\"",
        "id=\"mobile.app-title\"",
        "role=\"status\" aria-live=\"polite\"",
        "aria-label=\"Conversations\"",
        "for=\"mobile.tcp-endpoint\"",
        "for=\"mobile.draft.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        "for=\"mobile.delivery-method\"",
        "type=\"button\"",
    ] {
        assert!(markup.contains(required), "missing semantic contract: {required}");
    }

    assert!(!markup.contains("tabindex=\"1\""));
    assert!(!markup.contains("onclick="));
}

#[test]
fn shared_shell_exposes_typed_platform_facts_without_inventing_text_scale() {
    let platform_snapshot = PlatformSnapshot {
        generation: 3,
        sequence: 7,
        window: WindowMetrics {
            class: WindowClass::Compact,
            width_css_px: 390,
            height_css_px: 844,
        },
        accessibility: AccessibilityPreferences {
            text_scale: TextScale::Unavailable,
            appearance: Appearance::Dark,
            contrast: Contrast::Increased,
            motion: MotionPreference::Reduced,
        },
        geometry: PlatformGeometry {
            insets: PlatformInsets::CssEnvironment,
            keyboard: KeyboardGeometry::WebViewManaged { visible: true },
        },
        lifecycle: ApplicationLifecycle::Active,
        permissions: vec![PermissionStatus {
            kind: PermissionKind::Camera,
            state: AuthorizationState::Denied,
        }],
        notification_authorization: AuthorizationState::Granted,
    };
    let mut category_snapshot = platform_snapshot.clone();
    let ble_controls =
        BleControlState { permission: AuthorizationState::Restricted, ..Default::default() };
    let markup = dioxus_ssr::render_element(rsx! {
        PlatformShell {
            fixture: fixture("direct-message-queued"),
            platform_snapshot,
            ble_controls,
        }
    });

    for fact in [
        "data-window-class=\"compact\"",
        "data-appearance=\"dark\"",
        "data-contrast=\"increased\"",
        "data-motion=\"reduced\"",
        "data-text-scale=\"unavailable\"",
        "data-lifecycle=\"active\"",
        "data-keyboard-visible=\"true\"",
        "data-insets=\"css-environment\"",
    ] {
        assert!(markup.contains(fact), "missing platform fact: {fact}");
    }
    assert!(
        opening_tag_with_id(&markup, "mobile.permission.camera").contains("data-state=\"denied\"")
    );
    assert!(
        opening_tag_with_id(&markup, "mobile.permission.bluetooth")
            .contains("data-state=\"restricted\"")
    );
    assert!(
        opening_tag_with_id(&markup, "mobile.permission.notifications")
            .contains("data-state=\"granted\"")
    );
    assert!(
        opening_tag_with_id(&markup, "mobile.permission.location")
            .contains("data-state=\"not-requested\"")
    );
    assert!(markup.contains("Not requested by Styrene"));
    assert!(markup.contains("system Settings."));
    assert!(!opening_tag_with_id(&markup, "mobile.open-application-settings").contains("disabled"));
    assert!(
        opening_tag_with_id(&markup, "mobile.open-application-settings")
            .contains("aria-describedby=\"mobile.permissions-recovery\"")
    );
    assert!(!markup.contains("data-text-scale-percent"));

    category_snapshot.accessibility.text_scale =
        TextScale::Category(TextScaleCategory::AccessibilityExtraExtraExtraLarge);
    let category_markup = dioxus_ssr::render_element(rsx! {
        MobileShell {
            target: TargetClass::Ios,
            fixture: fixture("direct-message-queued"),
            platform_snapshot: category_snapshot,
        }
    });
    assert!(category_markup.contains("data-text-scale=\"category\""));
    assert!(
        category_markup
            .contains("data-text-scale-category=\"accessibility-extra-extra-extra-large\"")
    );
    assert!(!category_markup.contains("data-text-scale-percent"));
}

#[test]
fn mobile_shell_uses_destination_navigation_and_starts_on_the_conversation_list() {
    let markup = render(fixture("direct-message-queued"));

    for destination in ["messages", "people", "network", "more"] {
        assert!(markup.contains(&format!("id=\"mobile.destination.{destination}\"")));
    }
    assert!(
        opening_tag_with_id(&markup, "mobile.destination.messages")
            .contains("aria-current=\"page\"")
    );
    assert!(markup.contains("data-compact-pane=\"list\""));
    assert!(opening_tag_with_id(&markup, "mobile.people").contains("hidden"));
    assert!(opening_tag_with_id(&markup, "mobile.network").contains("hidden"));
    assert!(opening_tag_with_id(&markup, "mobile.more").contains("hidden"));
    let new_message = opening_tag_with_id(&markup, "mobile.new-message");
    assert!(new_message.contains("aria-expanded=\"false\""));
    assert!(new_message.contains("disabled"));
    assert!(new_message.contains("aria-describedby=\"mobile.new-message-disabled\""));
    assert!(markup.contains("New Message is unavailable in this view."));
}

fn render_new_message(initial_search: &str, initial_destination: &str) -> String {
    let state = fixture("canonical-peer-discovery");
    dioxus_ssr::render_element(rsx! {
        NewMessageShell {
            peers: state.peers,
            generation: state.generation,
            initial_search: initial_search.to_owned(),
            initial_destination: initial_destination.to_owned(),
        }
    })
}

#[test]
fn new_message_empty_entry_exposes_search_and_no_optimistic_product_state() {
    let markup = render_new_message("", "");
    let submit = opening_tag_with_id(&markup, "mobile.start-conversation");

    assert!(markup.contains("id=\"mobile.peer-search\""));
    assert!(opening_tag_with_id(&markup, "mobile.peer-search").contains("autofocus"));
    assert!(markup.contains("FPIG_SKYWAVE"));
    assert!(markup.contains("id=\"mobile.direct-destination\""));
    assert!(markup.contains("Enter a 32-character LXMF destination."));
    assert!(submit.contains("disabled"));
    assert!(!markup.contains("id=\"mobile.conversation.e01b09b22ccc4e2755d29eead962677b\""));
    assert!(!markup.contains("id=\"mobile.message."));
}

#[test]
fn new_message_search_filters_peer_choices_without_hiding_direct_entry() {
    let matching = render_new_message("skywave", "");
    let missing = render_new_message("not-present", "");

    assert!(matching.contains("id=\"mobile.new-message-peer.e01b09b22ccc4e2755d29eead962677b\""));
    assert!(!matching.contains("No discovered peers match"));
    assert!(missing.contains("No discovered peers match this search."));
    assert!(!missing.contains("id=\"mobile.new-message-peer.e01b09b22ccc4e2755d29eead962677b\""));
    assert!(missing.contains("id=\"mobile.direct-destination\""));
}

#[test]
fn new_message_bounded_candidate_uses_backend_validation_action_semantics() {
    let canonical = "e01b09b22ccc4e2755d29eead962677b";
    let valid = render_new_message("", canonical);
    let malformed_but_bounded = render_new_message("", &"z".repeat(32));

    assert!(!opening_tag_with_id(&valid, "mobile.start-conversation").contains("disabled"));
    assert!(valid.contains("maxlength=\"32\""));
    assert!(
        valid
            .contains("The backend will validate this destination before creating a conversation.")
    );
    assert!(
        !opening_tag_with_id(&malformed_but_bounded, "mobile.start-conversation")
            .contains("disabled")
    );
    assert!(malformed_but_bounded.contains("backend will validate"));
}

#[test]
fn new_message_clipboard_candidate_stays_on_backend_validation_path() {
    let state = fixture("canonical-peer-discovery");
    let candidate = "e01b09b22ccc4e2755d29eead962677b";
    let markup = dioxus_ssr::render_element(rsx! {
        NewMessageShell {
            peers: state.peers,
            generation: state.generation,
            initial_search: String::new(),
            initial_destination: candidate.to_owned(),
            paste_enabled: true,
        }
    });

    assert!(opening_tag_with_id(&markup, "mobile.paste-destination").contains("type=\"button\""));
    assert!(!opening_tag_with_id(&markup, "mobile.paste-destination").contains("disabled"));
    assert!(markup.contains("unvalidated destination candidate"));
    assert!(
        opening_tag_with_id(&markup, "mobile.direct-destination")
            .contains(&format!("value=\"{candidate}\""))
    );
    assert!(!opening_tag_with_id(&markup, "mobile.start-conversation").contains("disabled"));
    assert!(markup.contains("backend will validate"));
}

#[test]
fn new_message_clipboard_failure_is_bounded_and_associated() {
    let state = fixture("canonical-peer-discovery");
    let markup = dioxus_ssr::render_element(rsx! {
        NewMessageShell {
            peers: state.peers,
            generation: state.generation,
            initial_search: String::new(),
            initial_destination: String::new(),
            paste_failure: Some("denied".into()),
            paste_enabled: true,
        }
    });

    assert!(
        opening_tag_with_id(&markup, "mobile.paste-destination")
            .contains("aria-describedby=\"mobile.paste-destination-status\"")
    );
    assert!(markup.contains("Clipboard text was not added (denied)."));
}

#[test]
fn new_message_qr_capture_is_single_shot_and_uses_the_candidate_path() {
    let state = fixture("canonical-peer-discovery");
    let markup = dioxus_ssr::render_element(rsx! {
        NewMessageShell {
            peers: state.peers,
            generation: state.generation,
            initial_search: String::new(),
            initial_destination: String::new(),
            paste_enabled: true,
            scan_enabled: true,
        }
    });

    let capture = opening_tag_with_id(&markup, "mobile.scan-qr-input");
    assert!(capture.contains("type=\"file\""));
    assert!(capture.contains("accept=\"image/jpeg,image/png\""));
    assert!(capture.contains("capture=\"environment\""));
    assert!(!capture.contains("multiple"));
    assert!(markup.contains("Scan QR"));
    assert!(markup.contains("one QR code is treated as an unvalidated destination candidate"));
    assert!(!opening_tag_with_id(&markup, "mobile.paste-destination").contains("disabled"));
    assert!(opening_tag_with_id(&markup, "mobile.start-conversation").contains("disabled"));
}

#[test]
fn qr_denial_offers_settings_without_disabling_manual_or_paste_ingress() {
    let state = fixture("canonical-peer-discovery");
    let markup = dioxus_ssr::render_element(rsx! {
        NewMessageShell {
            peers: state.peers,
            generation: state.generation,
            initial_search: String::new(),
            initial_destination: String::new(),
            paste_enabled: true,
            scan_enabled: true,
            scan_failure: Some("denied".into()),
        }
    });

    assert!(markup.contains("QR image was not added (denied)."));
    assert!(markup.contains("Open system Settings"));
    assert!(!opening_tag_with_id(&markup, "mobile.paste-destination").contains("disabled"));
    assert!(!opening_tag_with_id(&markup, "mobile.direct-destination").contains("disabled"));
}

#[test]
fn qr_busy_state_disables_only_scan_capture() {
    let state = fixture("canonical-peer-discovery");
    let markup = dioxus_ssr::render_element(rsx! {
        NewMessageShell {
            peers: state.peers,
            generation: state.generation,
            initial_search: String::new(),
            initial_destination: String::new(),
            paste_enabled: true,
            scan_enabled: true,
            scan_busy: true,
        }
    });

    assert!(opening_tag_with_id(&markup, "mobile.scan-qr-input").contains("disabled"));
    assert!(markup.contains("Scanning QR"));
    assert!(!opening_tag_with_id(&markup, "mobile.paste-destination").contains("disabled"));
    assert!(!opening_tag_with_id(&markup, "mobile.direct-destination").contains("disabled"));
}

#[test]
fn new_message_rejects_incomplete_and_oversized_input_before_dispatch() {
    let incomplete = render_new_message("", "abc");
    let oversized = render_new_message("", &"a".repeat(4096));

    assert!(opening_tag_with_id(&incomplete, "mobile.start-conversation").contains("disabled"));
    assert!(incomplete.contains("must contain 32 characters"));
    assert!(opening_tag_with_id(&oversized, "mobile.start-conversation").contains("disabled"));
    assert!(
        opening_tag_with_id(&oversized, "mobile.direct-destination")
            .contains("aria-invalid=\"true\"")
    );
    assert!(opening_tag_with_id(&oversized, "mobile.direct-destination").contains("autofocus"));
    assert!(
        opening_tag_with_id(&oversized, "mobile.direct-destination")
            .contains("aria-errormessage=\"mobile.direct-destination-error\"")
    );
    assert!(opening_tag_with_id(&oversized, "mobile.direct-destination").contains(
        "aria-describedby=\"mobile.direct-destination-status mobile.direct-destination-error\""
    ));
    assert!(oversized.contains("id=\"mobile.direct-destination-error\""));
    assert!(oversized.contains("exceeds the 32-byte input limit"));
    assert!(!oversized.contains(&"a".repeat(34)));
}

#[test]
fn new_message_backend_failure_associates_and_focuses_the_destination_error() {
    let state = fixture("canonical-peer-discovery");
    let markup = dioxus_ssr::render_element(rsx! {
        NewMessageShell {
            peers: state.peers,
            generation: state.generation,
            initial_search: String::new(),
            initial_destination: "e01b09b22ccc4e2755d29eead962677b",
            failure: Some(TypedFailure {
                code: "conversation_start_failed".into(),
                retryable: true,
            }),
        }
    });

    let destination = opening_tag_with_id(&markup, "mobile.direct-destination");
    assert!(destination.contains("autofocus"));
    assert!(destination.contains("aria-invalid=\"true\""));
    assert!(destination.contains("aria-errormessage=\"mobile.new-message-failure\""));
    assert!(destination.contains(
        "aria-describedby=\"mobile.direct-destination-status mobile.new-message-failure\""
    ));
    assert!(markup.contains("id=\"mobile.new-message-failure\""));
    assert!(markup.contains("The backend rejected the destination. Check it and try again."));
}

#[test]
fn more_projects_secret_free_identity_custody() {
    let mut state = fixture("live-empty-connected");
    state.session.custody = Some(IdentityCustody {
        requested_backend: IdentityCustodyBackend::AndroidKeystore,
        active_backend: Some(IdentityCustodyBackend::AndroidKeystore),
        protection: Some(IdentityCustodyProtection::PlatformProtected),
        authentication: IdentityCustodyAuthentication::DeviceAuthentication,
        availability: IdentityCustodyAvailability::Available,
        downgrade: IdentityCustodyDowngrade::None,
        failure: None,
    });

    let markup = render(state);
    let custody = opening_tag_with_id(&markup, "mobile.identity-custody");

    assert!(custody.contains("aria-labelledby=\"mobile.identity-custody-heading\""));
    assert!(custody.contains("data-availability=\"available\""));
    assert!(custody.contains("data-downgrade=\"none\""));
    assert!(
        opening_tag_with_id(&markup, "mobile.identity-custody-active")
            .contains("data-backend=\"android_keystore\"")
    );
    assert!(markup.contains("Android Keystore"));
    assert!(markup.contains("Platform protected"));
    assert!(markup.contains("Device authentication"));
    assert!(!markup.contains("private_key"));
    assert!(!markup.contains("key_material"));
}

#[test]
fn more_discloses_unavailable_downgraded_identity_custody() {
    let mut state = fixture("live-empty-connected");
    state.session.custody = Some(IdentityCustody {
        requested_backend: IdentityCustodyBackend::Keychain,
        active_backend: Some(IdentityCustodyBackend::EncryptedFile),
        protection: Some(IdentityCustodyProtection::EncryptedAtRest),
        authentication: IdentityCustodyAuthentication::HostKeyMaterial,
        availability: IdentityCustodyAvailability::Unavailable,
        downgrade: IdentityCustodyDowngrade::ActiveBackendMismatch,
        failure: Some(TypedFailure { code: "backend_failure".into(), retryable: true }),
    });

    let markup = render(state);
    let custody = opening_tag_with_id(&markup, "mobile.identity-custody");
    let failure = opening_tag_with_id(&markup, "mobile.identity-custody-failure");

    assert!(custody.contains("data-availability=\"unavailable\""));
    assert!(custody.contains("data-downgrade=\"active_backend_mismatch\""));
    assert!(markup.contains("Active backend mismatch"));
    assert!(failure.contains("role=\"status\""));
    assert!(failure.contains("data-code=\"backend_failure\""));
    assert!(failure.contains("data-retryable=\"true\""));
}

#[test]
fn web_history_back_capability_exposes_only_a_hidden_rust_action() {
    let markup = dioxus_ssr::render_element(rsx! {
        MobileShell {
            target: TargetClass::Android,
            fixture: fixture("direct-message-queued"),
            back_navigation: BackNavigation::web_history(),
        }
    });

    let platform_back = opening_tag_with_id(&markup, "mobile.platform-back");
    assert!(platform_back.contains("hidden"));
    assert!(platform_back.contains("tabindex=\"-1\""));
}

#[test]
fn initial_thread_selection_filters_messages_without_inventing_ordering() {
    let mut state = fixture("direct-message-queued");
    let mut second_conversation = state.conversations[0].clone();
    second_conversation.peer_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    state.conversations.push(second_conversation);

    let mut second_message = state.messages[0].clone();
    second_message.id = "message-second-peer".into();
    second_message.peer_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    second_message.content = "Message belonging only to the second peer".into();
    state.messages.push(second_message);

    let markup = render(state);
    let first =
        opening_tag_with_id(&markup, "mobile.conversation.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let second =
        opening_tag_with_id(&markup, "mobile.conversation.bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    assert!(first.contains("aria-current=\"true\""));
    assert!(second.contains("aria-current=\"false\""));
    assert!(markup.contains("Direct message awaiting evidence"));
    assert!(!markup.contains("Message belonging only to the second peer"));
}

#[test]
fn composer_requires_a_canonical_conversation_before_enabling_send() {
    let state = fixture("live-empty-connected");
    let markup = dioxus_ssr::render_element(rsx! {
        Composer {
            conversation: None,
            enabled: true,
            propagation: PropagationUpdate::from_fixture(&state),
        }
    });

    let send = opening_tag_with_id(&markup, "mobile.send");
    assert!(send.contains("data-enabled=\"false\""));
    assert!(send.contains("disabled"));
}

#[test]
fn composer_disables_only_propagated_delivery_without_a_ready_node() {
    let mut state = fixture("direct-message-queued");
    state.conversations[0].draft = "Retain this draft".into();
    let conversation = state.conversations[0].clone();
    let mut propagation = PropagationUpdate::from_fixture(&state);
    propagation.selected_destination = None;
    propagation.ready = false;

    let markup = dioxus_ssr::render_element(rsx! {
        ComposerShell { conversation, propagation }
    });

    let direct = markup
        .split("<option")
        .find(|option| option.contains("value=\"direct\""))
        .expect("direct option");
    let propagated = markup
        .split("<option")
        .find(|option| option.contains("value=\"propagated\""))
        .expect("propagated option");

    assert!(!direct.split('>').next().unwrap().contains("disabled"));
    assert!(propagated.split('>').next().unwrap().contains("disabled"));
    assert!(markup.contains("Select a propagation node in Network"));
    assert!(opening_tag_with_id(&markup, "mobile.send").contains("data-enabled=\"true\""));
    assert!(!markup.contains("id=\"mobile.send-disabled-reason\""));
    assert!(markup.contains("Ready to send."));
}

#[test]
fn composer_enables_propagated_delivery_only_for_a_ready_selected_node() {
    let mut state = fixture("direct-message-queued");
    state.conversations[0].draft = "Retain this draft".into();
    let conversation = state.conversations[0].clone();
    let mut propagation = PropagationUpdate::from_fixture(&state);
    propagation.selected_destination = Some("feedfeedfeedfeedfeedfeedfeedfeed".into());
    propagation.ready = true;

    let markup = dioxus_ssr::render_element(rsx! {
        ComposerShell { conversation, propagation }
    });
    let propagated = markup
        .split("<option")
        .find(|option| option.contains("value=\"propagated\""))
        .expect("propagated option");

    assert!(!propagated.split('>').next().unwrap().contains("disabled"));
    assert!(markup.contains("Propagated delivery is available through the selected node."));
}

#[test]
fn malformed_short_hashes_do_not_crash_the_directory_or_conversation_list() {
    let mut state = fixture("direct-message-queued");
    state.conversations[0].peer_hash = "x".into();
    state.messages[0].peer_hash = "x".into();
    let markup = render(state);

    assert!(markup.contains("id=\"mobile.conversation.x\""));
    assert!(markup.contains("Peer x"));
}

#[test]
fn mobile_styles_cover_reflow_safe_areas_targets_and_preferences() {
    for required in [
        "--font-interface:",
        "--font-technical:",
        "--radius-control:",
        "--radius-panel:",
        "--radius-status:",
        "--size-touch-ios:",
        "--size-touch-android:",
        "--content-measure:",
        "--navigation-active:",
        "min-inline-size: 20rem",
        "font: -apple-system-body",
        "font: -apple-system-title2",
        "font: -apple-system-caption1",
        "[data-tone=\"positive\"]",
        "[data-tone=\"negative\"]",
        "button:disabled {",
        "min-block-size: 100dvh",
        "env(safe-area-inset-top)",
        "env(safe-area-inset-bottom)",
        "min-block-size: var(--size-touch-ios)",
        "[data-target=\"android\"] button",
        "min-block-size: var(--size-touch-android)",
        "flex-wrap: wrap",
        "overflow-wrap: anywhere",
        "@media (max-width: 30rem)",
        ".composer button,",
        ".new-message-actions > button",
        "@media (min-width: 52rem)",
        "@media (prefers-color-scheme: dark)",
        "@media (prefers-contrast: more)",
        "@media (prefers-reduced-motion: reduce)",
        "data-text-scale-category=\"accessibility-extra-extra-extra-large\"",
    ] {
        assert!(MOBILE_CSS.contains(required), "missing mobile style contract: {required}");
    }

    assert!(!MOBILE_CSS.contains("outline: none"));
    assert!(!MOBILE_CSS.contains("#mobile\\."));
    assert!(MOBILE_CSS.contains(".surface-card {"));
    assert!(MOBILE_CSS.contains(".primary-action {"));
    assert!(MOBILE_CSS.contains(
        ".destination-bar {\n  position: fixed;\n  inset-inline: 0;\n  inset-block-end: 0;"
    ));
    assert!(
        MOBILE_CSS
            .contains("--destination-bar-reserve: calc(4.5rem + env(safe-area-inset-bottom))")
    );
    assert!(MOBILE_CSS.contains("inset-block-end: var(--destination-bar-reserve)"));
    assert!(MOBILE_CSS.contains("[data-keyboard-visible=\"true\"] .destination-bar"));
}

#[test]
fn cold_restoration_renders_retained_message_and_draft_while_reconnecting() {
    let mut persisted = fixture("direct-message-queued");
    persisted.conversations[0].draft = "survives process death".into();

    let bytes = serde_json::to_vec(&persisted).expect("fixture must serialize");
    let persisted = serde_json::from_slice(&bytes).expect("fixture must restore");
    let store = MobileStore::cold_restore(persisted, 10);
    let markup = render(store.snapshot().clone());

    assert!(markup.contains("data-generation=\"10\""));
    assert!(markup.contains("data-phase=\"reconnecting\""));
    assert!(markup.contains("id=\"mobile.message.message-direct-1\""));
    assert!(markup.contains("Direct message awaiting evidence"));
    assert!(markup.contains("id=\"mobile.draft.aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\""));
    assert!(markup.contains("survives process death"));
}

#[test]
fn reconnect_component_keeps_content_and_reports_tcp_transition() {
    let initial = fixture("direct-message-queued");
    let mut store = MobileStore::new(initial.clone());
    store.begin_reconnect(5, "socket_closed");

    let reconnecting = render(store.snapshot().clone());
    assert!(reconnecting.contains("data-phase=\"reconnecting\""));
    assert!(reconnecting.contains("id=\"mobile.bearer.tcp\""));
    assert!(reconnecting.contains("data-state=\"reconnecting\""));
    assert!(reconnecting.contains("Direct message awaiting evidence"));

    let mut connected = initial;
    connected.generation = 5;
    connected.session.phase = SessionPhase::Connected;
    connected.bearers[0].state = BearerState::Connected;
    connected.bearers[0].reason = None;
    assert_eq!(store.apply_snapshot(5, connected), ApplyResult::Applied);

    let connected = render(store.snapshot().clone());
    assert!(connected.contains("data-phase=\"connected\""));
    assert!(connected.contains("data-state=\"connected\""));
}

#[test]
fn session_failure_exposes_a_typed_visible_status() {
    let markup = render(fixture("recoverable-session-failure"));
    let failure = opening_tag_with_id(&markup, "mobile.session-failure");

    assert!(failure.contains("role=\"status\""));
    assert!(failure.contains("data-code=\"invalid_tcp_endpoint\""));
    assert!(failure.contains("data-retryable=\"true\""));

    let mut connected = fixture("recoverable-session-failure");
    connected.session.phase = SessionPhase::Connected;
    let connected = render(connected);
    assert!(connected.contains("The last operation failed. Current session state is unchanged."));
    assert!(!connected.contains("Session unavailable. Open Network"));
}

#[test]
fn degraded_session_preserves_independent_runtime_and_phase() {
    let mut state = fixture("live-empty-connected");
    state.session.runtime = styrene_ui_state::SessionRuntime::Ready;
    state.session.phase = SessionPhase::Degraded;

    let markup = render(state);
    let status = opening_tag_with_id(&markup, "mobile.session-state");

    assert!(status.contains("data-runtime=\"ready\""));
    assert!(status.contains("data-phase=\"degraded\""));
    assert!(status.contains("data-tone=\"caution\""));
    assert!(markup.contains("Session degraded"));
    assert!(!markup.contains("Session reconnecting"));
}

#[test]
fn operational_summary_renders_bounded_authoritative_facts_and_unknown_routes() {
    let mut state = fixture("canonical-peer-discovery");
    state.conversations.push(Conversation {
        peer_hash: "e01b09b22ccc4e2755d29eead962677b".into(),
        unread_count: 3,
        draft: String::new(),
        draft_revision: 0,
    });
    let markup = render(state);

    assert!(markup.contains("id=\"mobile.operational-summary\""));
    assert!(markup.contains("Operational summary"));
    assert!(
        opening_tag_with_id(&markup, "mobile.summary.runtime").contains("data-phase=\"connected\"")
    );
    assert!(markup.contains("1 of 3 connected"));
    assert!(markup.contains("1 canonical observations"));
    assert!(markup.contains("id=\"mobile.summary.unread\">3"));
    assert!(markup.contains("Unknown; no loaded attempt evidence"));
    assert!(markup.contains("Selected node ready · idle"));
    assert!(markup.contains("loaded attempts only"));
    for fabricated in ["Relay connected", "Mail waiting", "Peer reachable"] {
        assert!(!markup.contains(fabricated));
    }
}

#[test]
fn operational_summary_preserves_reconnecting_degraded_failed_and_mixed_bearers() {
    let reconnecting = render(fixture("tcp-reconnecting-rnode-unavailable"));
    assert!(
        opening_tag_with_id(&reconnecting, "mobile.summary.runtime")
            .contains("data-phase=\"reconnecting\"")
    );
    assert!(reconnecting.contains("0 of 3 connected"));
    assert!(reconnecting.contains("Selected node not ready · idle"));

    let mut degraded_state = fixture("live-empty-connected");
    degraded_state.session.phase = SessionPhase::Degraded;
    let degraded = render(degraded_state);
    assert!(
        opening_tag_with_id(&degraded, "mobile.summary.runtime")
            .contains("data-phase=\"degraded\"")
    );

    let failed = render(fixture("recoverable-session-failure"));
    assert!(
        opening_tag_with_id(&failed, "mobile.summary.runtime").contains("data-phase=\"failed\"")
    );
}

#[test]
fn operational_summary_uses_the_current_propagation_projection_prop() {
    let fixture = fixture("live-empty-connected");
    let mut propagation = PropagationUpdate::from_fixture(&fixture);
    propagation.selected_destination = Some("feedfeedfeedfeedfeedfeedfeedfeed".into());
    propagation.ready = true;
    propagation.readiness = styrene_ui_state::PropagationReadiness::Ready;
    propagation.sync_state = SyncState::Complete;
    let markup = dioxus_ssr::render_element(rsx! {
        MobileShell {
            target: TargetClass::Ios,
            fixture,
            propagation,
        }
    });

    assert!(
        opening_tag_with_id(&markup, "mobile.summary.propagation")
            .contains("data-selected=\"true\"")
    );
    assert!(
        opening_tag_with_id(&markup, "mobile.summary.propagation").contains("data-ready=\"true\"")
    );
    assert!(markup.contains("Selected node ready · complete"));
}

#[test]
fn stale_completion_never_appears_in_rendered_state() {
    let mut store = MobileStore::new(fixture("stale-generation-rejected"));
    let mut stale = fixture("recoverable-session-failure");
    stale.generation = 7;

    assert_eq!(store.apply_snapshot(7, stale), ApplyResult::IgnoredStale);
    let markup = render(store.snapshot().clone());

    assert!(markup.contains("data-generation=\"8\""));
    assert!(markup.contains("data-phase=\"connected\""));
    assert!(markup.contains("data-state=\"connected\""));
    assert!(!markup.contains("invalid_tcp_endpoint"));
    assert!(!markup.contains("data-phase=\"failed\""));
}

#[test]
fn tcp_only_state_renders_messaging_as_enabled_without_rnode() {
    let store = MobileStore::new(fixture("direct-message-queued"));
    assert!(store.messaging_available());

    let markup = render(store.snapshot().clone());

    assert!(markup.contains("id=\"mobile.bearer.tcp\""));
    assert!(markup.contains("data-state=\"connected\""));
    assert!(markup.contains("id=\"mobile.bearer.bluetooth-rnode\""));
    assert!(markup.contains("id=\"mobile.bearer.bluetooth-rnode.state\""));
    assert!(markup.contains("aria-label=\"Bluetooth RNode bearer unavailable\""));
    assert!(markup.contains("data-state=\"unavailable\""));
    assert!(markup.contains("id=\"mobile.send\""));
    let send = opening_tag_with_id(&markup, "mobile.send");
    assert!(send.contains("data-enabled=\"false\""));
    assert!(send.contains("disabled"));
}

#[test]
fn network_renders_independent_denied_interrupted_and_unverified_bearers() {
    for (kind, state, reason, tone) in [
        ("bluetooth-rnode", "unavailable", "permission_denied", "neutral"),
        ("bluetooth-rnode", "disconnected", "connection_interrupted", "negative"),
        ("android-usb", "unverified", "physical_evidence_absent", "caution"),
    ] {
        let mut state_fixture = fixture("direct-message-queued");
        let bearer = state_fixture
            .bearers
            .iter_mut()
            .find(|bearer| bearer.kind.as_str() == kind)
            .expect("platform bearer");
        bearer.state = serde_json::from_str(&format!("\"{state}\"")).unwrap();
        bearer.reason = Some(reason.into());

        let markup = render(state_fixture);
        let tcp = opening_tag_with_id(&markup, "mobile.bearer.tcp");
        assert!(tcp.contains("data-state=\"connected\""));
        let bearer = opening_tag_with_id(&markup, &format!("mobile.bearer.{kind}"));
        assert!(bearer.contains(&format!("data-state=\"{state}\"")));
        assert!(bearer.contains(&format!("data-reason=\"{reason}\"")));
        let bearer_status = opening_tag_with_id(&markup, &format!("mobile.bearer.{kind}.state"));
        assert!(bearer_status.contains(&format!("data-tone=\"{tone}\"")));
        assert!(markup.contains("id=\"mobile.send\""));
        assert!(opening_tag_with_id(&markup, "mobile.send").contains("disabled"));
    }
}

#[test]
fn network_projection_exposes_the_backend_endpoint_as_an_editable_control() {
    let store = MobileStore::new(fixture("direct-message-queued"));
    let markup = render(store.snapshot().clone());

    assert!(markup.contains("id=\"mobile.tcp-endpoint\""));
    assert!(markup.contains("value=\"rns.styrene.io:4242\""));
    assert!(markup.contains("id=\"mobile.tcp-endpoint-apply\""));
    assert!(opening_tag_with_id(&markup, "mobile.tcp-endpoint-apply").contains("disabled"));
}

#[test]
fn repeated_announces_render_one_person_and_live_empty_renders_none() {
    let mut discovered = fixture("canonical-peer-discovery");
    discovered.profile = styrene_ui_state::Profile::Live;
    let directory = dioxus_ssr::render_element(rsx! { ActionShell { fixture: discovered } });
    let live_empty = render(fixture("live-empty-connected"));

    assert_eq!(directory.matches("id=\"mobile.peer.e01b09b22ccc4e2755d29eead962677b\"").count(), 1);
    assert!(directory.contains("FPIG_SKYWAVE"));
    assert!(directory.contains("lxmf.delivery"));
    assert!(directory.contains("Canonical announce"));
    assert!(directory.contains("observed 4s ago"));
    assert!(directory.contains("1 announce"));
    assert!(!directory.contains("reachable"));
    assert!(directory.contains("data-action=\"start-conversation\""));
    assert!(directory.contains("Start conversation"));
    assert!(
        !opening_tag_with_id(&directory, "mobile.peer.e01b09b22ccc4e2755d29eead962677b")
            .contains("disabled")
    );
    assert!(!directory.contains("No conversation yet"));
    assert!(!live_empty.contains("id=\"mobile.peer."));
    assert!(!live_empty.contains("FPIG_SKYWAVE"));
}

#[test]
fn old_peer_observation_exposes_age_without_claiming_reachability() {
    let mut state = fixture("canonical-peer-discovery");
    state.profile = styrene_ui_state::Profile::Live;
    state.peers[0].age_secs = 86_400;
    let markup = dioxus_ssr::render_element(rsx! { ActionShell { fixture: state } });

    assert!(markup.contains("observed 86400s ago"));
    assert!(!markup.to_ascii_lowercase().contains("reachable"));
    assert!(!markup.to_ascii_lowercase().contains("online"));
}

#[test]
fn identity_surface_labels_public_destination_and_exposes_durable_name_edit() {
    let mut state = fixture("live-empty-connected");
    state.session.display_name = "Field Node".into();
    let destination = state.session.identity_hash.clone();
    let markup = dioxus_ssr::render_element(rsx! { ActionShell { fixture: state } });

    assert!(markup.contains("Display name"));
    assert!(markup.contains("value=\"Field Node\""));
    assert!(markup.contains("This is the current public display name."));
    assert!(opening_tag_with_id(&markup, "mobile.identity-display-name-save").contains("disabled"));
    assert!(
        opening_tag_with_id(&markup, "mobile.identity-display-name-save")
            .contains("aria-describedby=\"mobile.identity-display-name-status\"")
    );
    assert!(markup.contains("Public LXMF destination"));
    assert!(markup.contains(&format!("aria-label=\"Public LXMF destination {destination}\"")));
    assert!(!markup.contains("Private key"));
}

#[test]
fn local_announce_status_discloses_local_acceptance_only() {
    let markup = dioxus_ssr::render_element(rsx! {
        LocalAnnounceStatus {
            outcome: LocalAnnounceOutcome {
                generation: 3,
                accepted_at: 1_787_927_100,
                local_dispatch_accepted: true,
                remote_reception_confirmed: false,
                failure: None,
            }
        }
    });

    assert!(markup.contains("Accepted by local transport"));
    assert!(markup.contains("Remote reception unconfirmed"));
    assert!(!markup.contains("Remote peer received"));
}

#[test]
fn messaging_components_distinguish_queue_upload_delivery_and_empty_live_state() {
    let queued = render(fixture("direct-message-queued"));
    let uploaded = render(fixture("propagation-uploaded-not-delivered"));
    let delivered = render(fixture("propagation-sync-complete"));
    let empty = render(fixture("live-empty-connected"));

    assert!(queued.contains("Accepted by local transport; recipient delivery pending"));
    assert!(!queued.contains(">Delivered<"));
    assert!(uploaded.contains("Uploaded to propagation node; recipient delivery pending"));
    assert!(!uploaded.contains(">Delivered<"));
    assert!(delivered.contains(">Delivered<"));
    assert!(empty.contains("id=\"mobile.messages-empty\""));
    assert!(empty.contains("No conversations yet"));
    assert!(!empty.contains("message-direct-1"));
}

#[test]
fn message_history_renders_direction_chronology_and_independent_delivery_evidence() {
    let mut state = fixture("direct-message-queued");
    let message = &mut state.messages[0];
    message.delivery = styrene_ui_state::DeliveryEvidence::Delivered;
    message.failure = None;
    message.details.source_hash = "local-source".into();
    message.details.destination_hash = message.peer_hash.clone();
    message.details.is_outgoing = true;
    message.details.timestamp = 1_700_000_000;
    message.details.requested_delivery_method = Some("propagated".into());
    message.details.actual_delivery_method = Some("direct".into());
    message.details.fallback_reason = Some("selected node stale".into());
    message.details.terminal_detail = Some("policy rejected retry".into());
    message.details.retry_eligible = Some(false);
    message.details.attempts.push(styrene_ui_state::MessageAttempt {
        number: 2,
        state: "failed".into(),
        bearer: Some("tcp".into()),
        route: styrene_ui_state::MessageRouteObservation {
            outcome: styrene_ui_state::MessageRouteOutcome::Observed,
            hops: Some(2),
            ..Default::default()
        },
        ..Default::default()
    });
    message.details.propagation_correlations.push(
        styrene_ui_state::MessagePropagationCorrelation {
            relation: "upload".into(),
            transient_id: "transient".into(),
            peer_hash: Some("feedfeedfeedfeedfeedfeedfeedfeed".into()),
            state: "accepted".into(),
            ..Default::default()
        },
    );
    message.details.delivery_evidence.push(styrene_ui_state::MessageDeliveryObservation {
        kind: styrene_ui_state::MessageDeliveryKind::PacketReceipt,
        state: styrene_ui_state::MessageDeliveryState::Completed,
        outcome: Some("delivered".into()),
        ..Default::default()
    });

    let markup = render(state);
    let card = opening_tag_with_id(&markup, "mobile.message.message-direct-1");

    assert!(card.contains("data-direction=\"outgoing\""));
    assert!(card.contains("data-timestamp=\"1700000000\""));
    assert!(card.contains("aria-labelledby=\"mobile.message-heading.message-direct-1\""));
    assert!(markup.contains("<h4 id=\"mobile.message-heading.message-direct-1\">Sent</h4>"));
    for expected in [
        "Sent",
        "Unix 1700000000",
        "Requested method: propagated",
        "Actual method: direct",
        "Fallback: selected node stale",
        "Terminal outcome: policy rejected retry",
        "Attempt 2: failed",
        "Bearer: tcp",
        "Route observed",
        "2 hops",
        "upload: accepted",
        "PacketReceipt: Completed",
        "Retry unavailable for this terminal outcome.",
    ] {
        assert!(markup.contains(expected), "missing message evidence: {expected}");
    }
    assert!(!markup.contains("id=\"mobile.retry.message-direct-1\""));
}

fn render_propagation(propagation: PropagationUpdate) -> String {
    dioxus_ssr::render_element(rsx! {
        PropagationPanel { propagation, actions_enabled: true }
    })
}

#[test]
fn propagation_component_discloses_selection_readiness_and_automatic_policy() {
    let fixture = fixture("canonical-peer-discovery");
    let mut propagation = PropagationUpdate::from_fixture(&fixture);
    propagation.automatic_sync_enabled = true;
    propagation.automatic_sync_cooldown_secs = 30;
    propagation.sync_deadline_secs = 32;
    propagation.trigger_capabilities = vec![
        PropagationTriggerSource::ForegroundOpportunity,
        PropagationTriggerSource::GrantedBackgroundOpportunity,
        PropagationTriggerSource::Manual,
    ];
    propagation.last_synchronization = Some(PropagationSynchronization {
        trigger: PropagationTriggerSource::Manual,
        started_at: 1_700_000_000,
        finished_at: 1_700_000_004,
        outcome: PropagationTerminalOutcome::Succeeded,
        new_messages: 2,
    });
    propagation.cooldown_remaining_secs = 12;
    let policy = PropagationPolicy {
        transfer_limit_kb: 256,
        sync_limit_kb: 4_000,
        stamp_cost: 16,
        stamp_flexibility: 3,
    };
    propagation.candidates = vec![
        PropagationCandidate {
            destination_hash: "780e7aa7b2f175c88f28c7ba8ab1b714".into(),
            active: true,
            observed_at: 1_787_927_000,
            age_secs: 4,
            policy: Some(policy.clone()),
        },
        PropagationCandidate {
            destination_hash: "99999999999999999999999999999999".into(),
            active: false,
            observed_at: 1_787_926_000,
            age_secs: 1_004,
            policy: Some(policy.clone()),
        },
    ];
    propagation.selected_policy = Some(policy);

    let markup = render_propagation(propagation);

    assert!(markup.contains("id=\"mobile.propagation-selected\""));
    assert!(markup.contains("780e7aa7b2f175c88f28c7ba8ab1b714"));
    assert!(markup.contains("data-ready=\"true\""));
    assert!(markup.contains("data-cooldown-secs=\"30\""));
    assert!(markup.contains("data-deadline-secs=\"32\""));
    assert!(markup.contains("connection, reconnection, and allowed foreground opportunities"));
    assert!(markup.contains("Background collection is best effort"));
    assert!(markup.contains("may consume network airtime"));
    assert!(markup.contains("foreground_opportunity, granted_background_opportunity, manual"));
    assert!(markup.contains("explicitly granted, not guaranteed"));
    assert!(
        opening_tag_with_id(&markup, "mobile.propagation-last-sync")
            .contains("data-trigger=\"manual\"")
    );
    assert!(
        opening_tag_with_id(&markup, "mobile.propagation-last-sync")
            .contains("data-outcome=\"succeeded\"")
    );
    assert!(markup.contains("Last synchronization: manual · succeeded · 2 new messages"));
    assert!(
        opening_tag_with_id(&markup, "mobile.propagation-cooldown")
            .contains("data-remaining-secs=\"12\"")
    );
    assert!(markup.contains("id=\"mobile.propagation-sync\""));
    assert!(markup.contains("id=\"mobile.propagation-node\""));
    assert!(markup.contains("value=\"780e7aa7b2f175c88f28c7ba8ab1b714\" selected"));
    assert!(markup.contains("value=\"99999999999999999999999999999999\" disabled"));
    assert!(markup.contains("id=\"mobile.propagation-policy\""));
    assert!(markup.contains("data-stamp-cost=\"16\""));
    for excluded in ["propagation-host", "peering-control", "capacity-control", "expiry-control"] {
        assert!(!markup.contains(excluded));
    }
}

#[test]
fn stale_propagation_metadata_disables_manual_sync_without_losing_selection() {
    let fixture = fixture("tcp-reconnecting-rnode-unavailable");
    let markup = render_propagation(PropagationUpdate::from_fixture(&fixture));

    assert!(markup.contains("780e7aa7b2f175c88f28c7ba8ab1b714"));
    assert!(markup.contains("data-ready=\"false\""));
    assert!(opening_tag_with_id(&markup, "mobile.propagation-sync").contains("disabled"));
    assert!(
        opening_tag_with_id(&markup, "mobile.propagation-node")
            .contains("aria-describedby=\"mobile.propagation-node-disabled\"")
    );
    assert!(markup.contains("Propagation-node selection is unavailable in this view."));
    assert!(markup.contains("id=\"mobile.propagation-sync-disabled\""));
    assert!(markup.contains("The selected propagation node is not ready."));
    assert!(markup.contains("Selected node is currently unavailable"));
}

#[test]
fn propagation_component_renders_progress_repeat_sync_and_recoverable_failure() {
    let completed_fixture = fixture("propagation-sync-complete");
    let mut progress = PropagationUpdate::from_fixture(&completed_fixture);
    progress.sync_state = SyncState::InProgress;
    progress.progress = Some(PropagationProgress {
        attempt_id: "attempt-mobile-sync".into(),
        received_count: 1,
        received_bytes: 256,
    });
    progress.active_trigger = Some(PropagationTriggerSource::ForegroundOpportunity);
    progress.active_sync_started_at = Some(1_700_000_100);
    let progress_markup = render_propagation(progress);
    assert!(progress_markup.contains("id=\"mobile.propagation-progress\""));
    assert!(progress_markup.contains("data-attempt-id=\"attempt-mobile-sync\""));
    assert!(progress_markup.contains("data-received-count=\"1\""));
    assert!(
        opening_tag_with_id(&progress_markup, "mobile.propagation-active-trigger")
            .contains("data-trigger=\"foreground_opportunity\"")
    );

    let mut repeated = PropagationUpdate::from_fixture(&completed_fixture);
    repeated.new_messages = 0;
    let repeated_markup = render_propagation(repeated);
    assert!(repeated_markup.contains("id=\"mobile.propagation-result\""));
    assert!(repeated_markup.contains("0 new messages"));

    let failed_fixture = fixture("recoverable-session-failure");
    let mut failed = PropagationUpdate::from_fixture(&failed_fixture);
    failed.failure = Some(TypedFailure { code: "transport_unavailable".into(), retryable: true });
    let failure_markup = render_propagation(failed);
    assert!(opening_tag_with_id(&failure_markup, "mobile.propagation-sync").contains("autofocus"));
    assert!(failure_markup.contains("id=\"mobile.propagation-failure\""));
    assert!(failure_markup.contains("data-code=\"transport_unavailable\""));
    assert!(failure_markup.contains("data-retryable=\"true\""));
}

#[test]
fn composer_projects_backend_draft_revision_and_retryability() {
    let mut fixture = fixture("direct-message-queued");
    fixture.conversations[0].draft = "newer draft".into();
    fixture.conversations[0].draft_revision = 7;
    fixture.messages[0].failure = Some(styrene_ui_state::TypedFailure {
        code: "transport_unavailable".into(),
        retryable: true,
    });
    let markup = render(fixture);

    assert!(markup.contains("id=\"mobile.composer\""));
    assert!(markup.contains("data-revision=7"));
    assert!(markup.contains("newer draft"));
    assert!(markup.contains("id=\"mobile.delivery-method\""));
    assert!(opening_tag_with_id(&markup, "mobile.send").contains("type=\"submit\""));
    assert!(markup.contains("id=\"mobile.retry.message-direct-1\""));
    assert!(opening_tag_with_id(&markup, "mobile.retry.message-direct-1").contains("disabled"));
}

#[test]
fn composer_exposes_disabled_send_reason_when_completion_is_blocked() {
    let state = fixture("live-empty-connected");
    let markup = dioxus_ssr::render_element(rsx! {
        Composer {
            conversation: None,
            enabled: true,
            propagation: PropagationUpdate::from_fixture(&state),
        }
    });

    let send = opening_tag_with_id(&markup, "mobile.send");
    assert!(
        send.contains("aria-describedby=\"mobile.composer-status mobile.send-disabled-reason\"")
    );
    assert!(markup.contains("id=\"mobile.send-disabled-reason\""));
    assert!(markup.contains("Choose a conversation before sending a message."));
}
