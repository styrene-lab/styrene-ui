use dioxus::prelude::*;
use styrene_ui_app::{BackNavigation, MobileShell};
#[cfg(all(
    any(target_os = "android", target_os = "ios", target_os = "macos"),
    not(feature = "ui-test")
))]
use styrene_ui_platform::{AndroidUsbAttachment, AuthorizationState};
use styrene_ui_state::TargetClass;
#[cfg(any(
    test,
    feature = "ui-test",
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
use styrene_ui_state::{MobileFixture, MobileMinimumCorpus};

#[cfg(all(target_os = "android", not(feature = "ui-test")))]
mod android_usb;
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
        let update_receiver = session.read().clone();
        use_future(move || {
            let update_receiver = update_receiver.clone();
            async move {
                while let Some(next) = update_receiver.next_update().await {
                    update.set(next);
                }
            }
        });
        let current = update.read().clone();

        return rsx! {
            MobileShell {
                target: target_class(),
                fixture: current.fixture,
                propagation: current.propagation,
                platform_snapshot: platform_snapshot.read().clone(),
                back_navigation: BackNavigation::web_history(),
                action_sink: move |action| session.read().dispatch(action),
                android_usb_attachments: usb_attachments.read().clone(),
                android_usb_authorization: *usb_authorization.read(),
                android_usb_failure: usb_failure.read().clone(),
                android_usb_busy: *usb_busy.read(),
                android_usb_probe_status: usb_probe_status.read().clone(),
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
            }
        };
    }

    #[cfg(any(
        feature = "ui-test",
        not(any(target_os = "android", target_os = "ios", target_os = "macos"))
    ))]
    rsx! {
        MobileShell {
            target: target_class(),
            fixture: bootstrap_fixture(),
            platform_snapshot: platform_snapshot.read().clone(),
            back_navigation: BackNavigation::web_history(),
        }
    }
}

#[cfg(test)]
mod tests {
    use styrene_ui_state::{Profile, RuntimeBoundary};

    use super::*;

    const BACKEND_REVISION: &str = "0d3fc6ead37ab3a6857f825c260fd62f47977f55";
    const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");
    const MOBILE_MANIFEST: &str = include_str!("../Cargo.toml");
    const FIXTURE_PROVENANCE: &str =
        include_str!("../../../tests/fixtures/mobile-minimum-v1/README.md");
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
        assert_eq!(MOBILE_MANIFEST.matches(BACKEND_REVISION).count(), 4);
        assert!(FIXTURE_PROVENANCE.contains(BACKEND_REVISION));
        assert!(PARITY_CONTRACT.contains(BACKEND_REVISION));
        assert!(PARITY_CONTRACT.contains("styrene-mobile-integration-v1"));
        assert!(PARITY_CONTRACT.contains("Not consumed by UI"));
    }
}
