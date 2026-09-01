use dioxus::prelude::*;
use styrene_ui_app::{BackNavigation, MobileShell};
use styrene_ui_platform::ClipboardTextWriter;
#[cfg(all(
    any(target_os = "android", target_os = "ios", target_os = "macos"),
    not(feature = "ui-test")
))]
use styrene_ui_platform::{
    AndroidUsbAttachment, ApplicationSettingsService, AuthorizationState, BleControlPhase,
    BleControlState, ClipboardTextReader, TextAcquisitionGeneration,
};
#[cfg(all(
    not(target_os = "ios"),
    any(target_os = "android", target_os = "macos"),
    not(feature = "ui-test")
))]
use styrene_ui_platform::{BleAdapterState, BleControlFailure, PermissionKind};
use styrene_ui_state::TargetClass;
#[cfg(any(
    test,
    feature = "ui-test",
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
use styrene_ui_state::{MobileFixture, MobileMinimumCorpus};

#[cfg(all(target_os = "android", not(feature = "ui-test")))]
mod android_usb;
pub mod ble_session;
#[cfg(all(target_os = "ios", not(feature = "ui-test")))]
mod ios_ble;
mod platform;
#[cfg(all(
    any(target_os = "android", target_os = "ios", target_os = "macos"),
    not(feature = "ui-test")
))]
mod session;

#[cfg(any(
    test,
    feature = "ui-test",
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
const FIXTURES: &str = include_str!("../../../tests/fixtures/mobile-minimum-v1/states.json");
#[cfg(any(
    test,
    feature = "ui-test",
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
const BOOTSTRAP_FIXTURE: &str = "propagation-sync-complete";
pub const MOBILE_INDEX: &str = r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover, interactive-widget=resizes-content">
        <meta name="color-scheme" content="light dark">
    </head>
    <body>
        <div id="main"></div>
    </body>
</html>"#;

fn target_class() -> TargetClass {
    if cfg!(target_os = "android") { TargetClass::Android } else { TargetClass::Ios }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IdentityCopyCompletion {
    generation: u64,
    destination: String,
    failure: Option<String>,
}

impl IdentityCopyCompletion {
    fn is_for(&self, generation: u64, destination: &str) -> bool {
        self.generation == generation && self.destination == destination
    }
}

#[cfg(any(
    test,
    feature = "ui-test",
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
fn bootstrap_fixture() -> MobileFixture {
    #[cfg(feature = "ui-test")]
    let requested = std::env::var("STYRENE_UI_FIXTURE_ID").ok();
    #[cfg(not(feature = "ui-test"))]
    let requested = None::<String>;
    let fixture_id = requested.as_deref().unwrap_or(BOOTSTRAP_FIXTURE);
    let corpus: MobileMinimumCorpus =
        serde_json::from_str(FIXTURES).expect("embedded mobile fixture corpus must deserialize");
    corpus
        .fixtures
        .into_iter()
        .find(|fixture| fixture.id == fixture_id)
        .unwrap_or_else(|| panic!("embedded mobile fixture {fixture_id} must exist"))
}

#[component]
pub fn App() -> Element {
    platform::use_back_navigation();
    let platform_snapshot = platform::use_platform_snapshot();
    let mut identity_copy_busy = use_signal(|| false);
    let mut identity_copy_completion = use_signal(|| None::<IdentityCopyCompletion>);
    #[cfg(all(
        any(target_os = "android", target_os = "ios", target_os = "macos"),
        not(feature = "ui-test")
    ))]
    let (
        mut usb_attachments,
        mut usb_authorization,
        mut usb_failure,
        mut usb_busy,
        mut selected_usb,
        mut usb_probe_status,
    ) = (
        use_signal(Vec::<AndroidUsbAttachment>::new),
        use_signal(|| None::<AuthorizationState>),
        use_signal(|| None::<String>),
        use_signal(|| false),
        use_signal(|| None::<AndroidUsbAttachment>),
        use_signal(|| None::<String>),
    );
    #[cfg(all(
        any(target_os = "android", target_os = "ios", target_os = "macos"),
        not(feature = "ui-test")
    ))]
    let (mut clipboard_candidate, mut clipboard_failure, mut clipboard_busy) =
        (use_signal(|| None::<String>), use_signal(|| None::<String>), use_signal(|| false));
    #[cfg(all(
        any(target_os = "android", target_os = "ios", target_os = "macos"),
        not(feature = "ui-test")
    ))]
    let (mut application_settings_busy, mut application_settings_failure) =
        (use_signal(|| false), use_signal(|| None::<String>));

    #[cfg(all(target_os = "android", not(feature = "ui-test")))]
    use_future(move || async move {
        loop {
            match platform::android_usb_attachments().await {
                Ok(attachments) => {
                    let selected = selected_usb.read().clone();
                    if selected.as_ref().is_some_and(|selected| !attachments.contains(selected)) {
                        selected_usb.set(None);
                        usb_authorization.set(None);
                        usb_probe_status.set(None);
                    }
                    usb_attachments.set(attachments);
                }
                Err(error) => usb_failure.set(Some(error.code)),
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    #[cfg(all(
        any(target_os = "android", target_os = "ios", target_os = "macos"),
        not(feature = "ui-test")
    ))]
    {
        let session = use_signal(session::MobileSession::start);
        let mut update = use_signal(session::MobileSession::starting_update);
        #[cfg(target_os = "ios")]
        let ble_host = use_signal(ios_ble::IosBleHost::new);
        #[cfg(target_os = "ios")]
        let mut native_ble_controls = use_signal(BleControlState::default);
        let update_receiver = session.read().clone();
        use_future(move || {
            let update_receiver = update_receiver.clone();
            async move {
                while let Some(next) = update_receiver.next_update().await {
                    update.set(next);
                }
            }
        });
        #[cfg(target_os = "ios")]
        {
            let runner = ble_host.read().clone();
            let backend = session.read().clone();
            use_future(move || {
                let runner = runner.clone();
                let backend = backend.clone();
                async move { runner.run(backend).await }
            });
            let updates = ble_host.read().clone();
            use_future(move || {
                let updates = updates.clone();
                async move {
                    while let Some(next) = updates.next_update().await {
                        native_ble_controls.set(next);
                    }
                }
            });
        }
        let current = update.read().clone();
        let current_generation = current.fixture.generation;
        let current_platform = platform_snapshot.read().clone();
        #[cfg(not(target_os = "ios"))]
        let bluetooth_permission = current_platform
            .as_ref()
            .and_then(|snapshot| {
                snapshot
                    .permissions
                    .iter()
                    .find(|permission| permission.kind == PermissionKind::Bluetooth)
            })
            .map_or(AuthorizationState::Unavailable, |permission| permission.state);
        #[cfg(not(target_os = "ios"))]
        let ble_controls = BleControlState {
            permission: bluetooth_permission,
            adapter: BleAdapterState::Unavailable,
            phase: BleControlPhase::Idle,
            candidates: Vec::new(),
            approved: None,
            failure: Some(BleControlFailure::PlatformUnavailable),
            diagnostic_code: Some("platform_unavailable".into()),
        };
        #[cfg(target_os = "ios")]
        let ble_controls = {
            let mut controls = native_ble_controls.read().clone();
            if let Some(bearer) = current
                .fixture
                .bearers
                .iter()
                .find(|bearer| bearer.kind == styrene_ui_state::BearerKind::BluetoothRnode)
                .filter(|_| controls.approved.is_some() && controls.failure.is_none())
            {
                controls.phase = match bearer.state {
                    styrene_ui_state::BearerState::Connected => BleControlPhase::Connected,
                    styrene_ui_state::BearerState::Reconnecting => BleControlPhase::Reconnecting,
                    _ => controls.phase,
                };
            }
            controls
        };
        let usb_probe_ready = {
            let selected = selected_usb.read();
            let attachments = usb_attachments.read();
            *usb_authorization.read() == Some(AuthorizationState::Granted)
                && selected.as_ref().is_some_and(|selected| attachments.contains(selected))
        };
        let current_destination = current.fixture.session.identity_hash.clone();
        let visible_copy_completion = identity_copy_completion
            .read()
            .clone()
            .filter(|completion| completion.is_for(current_generation, &current_destination));
        let identity_copy_succeeded =
            visible_copy_completion.as_ref().is_some_and(|completion| completion.failure.is_none());
        let identity_copy_failure =
            visible_copy_completion.and_then(|completion| completion.failure);

        return rsx! {
            MobileShell {
                target: target_class(),
                fixture: current.fixture,
                propagation: current.propagation,
                platform_snapshot: current_platform,
                ble_controls,
                back_navigation: BackNavigation::web_history(),
                action_sink: move |action| session.read().dispatch(action),
                android_usb_attachments: usb_attachments.read().clone(),
                android_usb_authorization: *usb_authorization.read(),
                android_usb_failure: usb_failure.read().clone(),
                android_usb_busy: *usb_busy.read(),
                android_usb_probe_status: usb_probe_status.read().clone(),
                android_usb_probe_ready: usb_probe_ready,
                android_usb_refresh: move |()| {
                    if *usb_busy.read() {
                        return;
                    }
                    usb_busy.set(true);
                    spawn(async move {
                        usb_failure.set(None);
                        match platform::android_usb_attachments().await {
                            Ok(attachments) => usb_attachments.set(attachments),
                            Err(error) => usb_failure.set(Some(error.code)),
                        }
                        usb_busy.set(false);
                    });
                },
                android_usb_select: move |attachment: AndroidUsbAttachment| {
                    if *usb_busy.read() {
                        return;
                    }
                    usb_busy.set(true);
                    selected_usb.set(Some(attachment.clone()));
                    usb_probe_status.set(None);
                    let session = session.read().clone();
                    spawn(async move {
                        usb_failure.set(None);
                        usb_authorization.set(Some(AuthorizationState::NotDetermined));
                        if let Err(error) = session.request_android_usb_fallback().await {
                            usb_failure.set(Some(error));
                            usb_authorization.set(None);
                            selected_usb.set(None);
                            usb_busy.set(false);
                            return;
                        }
                        match platform::request_android_usb_authorization(attachment.clone()).await {
                            Ok(authorization) => {
                                usb_authorization.set(Some(authorization));
                                if authorization == AuthorizationState::Granted
                                    && let Err(error) = session.connect_android_usb(attachment).await
                                {
                                    usb_failure.set(Some(error));
                                } else if authorization == AuthorizationState::Denied
                                    && let Err(error) =
                                        session.report_android_usb_permission_denied().await
                                {
                                    usb_failure.set(Some(error));
                                }
                            }
                            Err(error) => {
                                usb_failure.set(Some(error.code));
                                usb_authorization.set(None);
                            }
                        }
                        if let Ok(attachments) = platform::android_usb_attachments().await {
                            usb_attachments.set(attachments);
                        }
                        usb_busy.set(false);
                    });
                },
                android_usb_probe: move |()| {
                    if *usb_busy.read() {
                        return;
                    }
                    usb_busy.set(true);
                    let session = session.read().clone();
                    spawn(async move {
                        usb_probe_status.set(Some("Dispatching local announce to the USB RNode".into()));
                        match session.probe_android_usb().await {
                            Ok(outcome) => usb_probe_status.set(Some(format!(
                                "USB accepted a {}-byte KISS frame. RF and remote reception unconfirmed.",
                                outcome.frame_bytes
                            ))),
                            Err(error) => usb_probe_status.set(Some(format!(
                                "USB RNode packet test failed: {error}"
                            ))),
                        }
                        usb_busy.set(false);
                    });
                },
                ble_scan: move |()| {
                    #[cfg(target_os = "ios")]
                    ble_host.read().scan();
                },
                ble_select: move |id| {
                    #[cfg(target_os = "ios")]
                    ble_host.read().select(id);
                    #[cfg(not(target_os = "ios"))]
                    let _ = id;
                },
                ble_retry: move |()| {
                    #[cfg(target_os = "ios")]
                    ble_host.read().retry();
                },
                ble_forget: move |()| {
                    #[cfg(target_os = "ios")]
                    ble_host.read().forget();
                },
                clipboard_candidate: clipboard_candidate.read().clone(),
                clipboard_failure: clipboard_failure.read().clone(),
                clipboard_busy: *clipboard_busy.read(),
                clipboard_read: move |()| {
                    if *clipboard_busy.read() {
                        return;
                    }
                    clipboard_busy.set(true);
                    clipboard_failure.set(None);
                    let generation = TextAcquisitionGeneration::new(current_generation);
                    spawn(async move {
                        let reader = platform::NativeClipboardTextReader;
                        let completion = reader.read_clipboard_text(generation).await;
                        if let Some(result) = completion.into_result_for(generation) {
                            match result {
                                Ok(candidate) => {
                                    clipboard_candidate.set(Some(candidate.into_string()));
                                }
                                Err(error) => {
                                    clipboard_failure.set(Some(
                                        format!("{error:?}").to_ascii_lowercase(),
                                    ));
                                }
                            }
                        }
                        clipboard_busy.set(false);
                    });
                },
                identity_copy_busy: *identity_copy_busy.read(),
                identity_copy_succeeded,
                identity_copy_failure,
                identity_copy: move |value: String| {
                    if *identity_copy_busy.read() {
                        return;
                    }
                    identity_copy_busy.set(true);
                    identity_copy_completion.set(None);
                    spawn(async move {
                        let writer = platform::NativeClipboardTextWriter;
                        let destination = value.clone();
                        let failure = writer.write_clipboard_text(value).await.err().map(|error| error.code);
                        identity_copy_completion.set(Some(IdentityCopyCompletion {
                            generation: current_generation,
                            destination,
                            failure,
                        }));
                        identity_copy_busy.set(false);
                    });
                },
                application_settings_busy: *application_settings_busy.read(),
                application_settings_failure: application_settings_failure.read().clone(),
                open_application_settings: move |()| {
                    if *application_settings_busy.read() {
                        return;
                    }
                    application_settings_busy.set(true);
                    application_settings_failure.set(None);
                    spawn(async move {
                        let service = platform::NativeApplicationSettingsService;
                        if let Err(error) = service.open_application_settings().await {
                            application_settings_failure.set(Some(error.code));
                        }
                        application_settings_busy.set(false);
                    });
                },
            }
        };
    }

    #[cfg(any(
        feature = "ui-test",
        not(any(target_os = "android", target_os = "ios", target_os = "macos"))
    ))]
    {
        let fixture = bootstrap_fixture();
        let current_generation = fixture.generation;
        let current_destination = fixture.session.identity_hash.clone();
        let visible_copy_completion = identity_copy_completion
            .read()
            .clone()
            .filter(|completion| completion.is_for(current_generation, &current_destination));
        let identity_copy_succeeded =
            visible_copy_completion.as_ref().is_some_and(|completion| completion.failure.is_none());
        let identity_copy_failure =
            visible_copy_completion.and_then(|completion| completion.failure);

        rsx! {
        MobileShell {
            target: target_class(),
            fixture,
            platform_snapshot: platform_snapshot.read().clone(),
            back_navigation: BackNavigation::web_history(),
            identity_copy_busy: *identity_copy_busy.read(),
            identity_copy_succeeded,
            identity_copy_failure,
            identity_copy: move |value: String| {
                if *identity_copy_busy.read() {
                    return;
                }
                identity_copy_busy.set(true);
                identity_copy_completion.set(None);
                spawn(async move {
                    let writer = platform::NativeClipboardTextWriter;
                    let destination = value.clone();
                    let failure = writer.write_clipboard_text(value).await.err().map(|error| error.code);
                    identity_copy_completion.set(Some(IdentityCopyCompletion {
                        generation: current_generation,
                        destination,
                        failure,
                    }));
                    identity_copy_busy.set(false);
                });
            },
        }
        }
    }
}

#[cfg(test)]
mod tests {
    use styrene_ui_state::{Profile, RuntimeBoundary};

    use super::*;

    #[test]
    fn identity_copy_completion_is_bound_to_generation_and_destination() {
        let completion =
            IdentityCopyCompletion { generation: 7, destination: "aabbccdd".into(), failure: None };

        assert!(completion.is_for(7, "aabbccdd"));
        assert!(!completion.is_for(8, "aabbccdd"));
        assert!(!completion.is_for(7, "eeff0011"));
    }

    const BACKEND_REVISION: &str = "f0359c92ba9f6d63ce248bc97617fc3115a0f3c3";
    const MINIMUM_FIXTURE_REVISION: &str = "899da81302c5f4e92f60a2fdaf396c26e813ba76";
    const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");
    const MOBILE_MANIFEST: &str = include_str!("../Cargo.toml");
    const FIXTURE_PROVENANCE: &str =
        include_str!("../../../tests/fixtures/mobile-minimum-v1/README.md");
    const APPLICATION_PROVENANCE: &str =
        include_str!("../../../tests/fixtures/mobile-application-parity-v1/README.md");
    const HANDOFF_PROVENANCE: &str =
        include_str!("../../../tests/fixtures/mobile-product-handoff-v1/README.md");
    const INTEGRATION_CORPUS: &str =
        include_str!("../../../tests/fixtures/mobile-integration-v1/corpus.json");
    const INTEGRATION_PROVENANCE: &str =
        include_str!("../../../tests/fixtures/mobile-integration-v1/README.md");
    const PARITY_CONTRACT: &str = include_str!("../../../docs/parity-corpus.md");

    #[test]
    fn embedded_fixture_corpus_remains_visibly_fixture_only() {
        let fixture = bootstrap_fixture();

        assert_eq!(fixture.id, BOOTSTRAP_FIXTURE);
        assert_eq!(fixture.profile, Profile::Fixture);
        assert!(RuntimeBoundary::from(fixture.profile).fixture_marker_visible());
        assert!(!RuntimeBoundary::from(fixture.profile).live_network_allowed());
    }

    #[test]
    fn mobile_index_preserves_zoom_and_covers_safe_areas() {
        assert!(MOBILE_INDEX.contains("<html lang=\"en\">"));
        assert!(MOBILE_INDEX.contains("width=device-width"));
        assert!(MOBILE_INDEX.contains("viewport-fit=cover"));
        assert!(MOBILE_INDEX.contains("name=\"color-scheme\" content=\"light dark\""));
        assert!(!MOBILE_INDEX.contains("user-scalable=no"));
        assert!(!MOBILE_INDEX.contains("maximum-scale"));
    }

    #[test]
    fn workspace_and_corpora_share_the_backend_contract() {
        assert!(WORKSPACE_MANIFEST.contains("resolver = \"3\""));
        assert!(WORKSPACE_MANIFEST.contains("edition = \"2024\""));
        assert_eq!(WORKSPACE_MANIFEST.matches(BACKEND_REVISION).count(), 6);
        assert_eq!(MOBILE_MANIFEST.matches(BACKEND_REVISION).count(), 4);
        assert!(FIXTURE_PROVENANCE.contains(MINIMUM_FIXTURE_REVISION));
        assert!(HANDOFF_PROVENANCE.contains(BACKEND_REVISION));
        assert!(INTEGRATION_PROVENANCE.contains("0bcf5843208a9a2578836e26b4ac4e23a0f7b4e7"));
        let integration: serde_json::Value =
            serde_json::from_str(INTEGRATION_CORPUS).expect("integration corpus must deserialize");
        let identity_case = integration["cases"]
            .as_array()
            .expect("integration corpus cases")
            .iter()
            .find(|case| case["id"] == "mobile.identity.copy-public-destination")
            .expect("public destination case");
        assert_eq!(
            identity_case["actions"],
            serde_json::json!(["open-identity", "copy-public-hash", "read-clipboard"])
        );
        assert!(APPLICATION_PROVENANCE.contains("Source baseline revision:"));
        assert!(APPLICATION_PROVENANCE.contains("before using the UI copy as revision-locked"));
        assert!(PARITY_CONTRACT.contains("current integration baseline revision"));
        assert!(PARITY_CONTRACT.contains("uncommitted Skywave build 9 candidate"));
        assert!(PARITY_CONTRACT.contains("styrene-mobile-integration-v1"));
        assert!(PARITY_CONTRACT.contains("Not consumed by UI"));
    }
}
