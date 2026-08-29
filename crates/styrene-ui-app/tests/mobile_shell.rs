use dioxus::prelude::*;
use styrene_ui_app::{LocalAnnounceStatus, MobileShell, PropagationPanel};
use styrene_ui_state::{
    ApplyResult, BearerState, LocalAnnounceOutcome, MobileFixture, MobileMinimumCorpus,
    MobileStore, PropagationCandidate, PropagationPolicy, PropagationProgress, PropagationUpdate,
    RuntimeBoundary, SessionPhase, SyncState, TargetClass, TypedFailure,
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

fn opening_tag_with_id<'a>(markup: &'a str, id: &str) -> &'a str {
    let id = format!("id=\"{id}\"");
    let start = markup.find(&id).unwrap_or_else(|| panic!("missing {id}"));
    let end = start + markup[start..].find('>').expect("element opening tag") + 1;
    &markup[start..end]
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
fn mobile_shell_uses_destination_navigation_and_starts_on_the_conversation_list() {
    let markup = render(fixture("direct-message-queued"));

    for destination in ["messages", "people", "network", "more"] {
        assert!(markup.contains(&format!("id=\"mobile.destination.{destination}\"")));
    }
    assert!(opening_tag_with_id(&markup, "mobile.destination.messages")
        .contains("aria-current=\"page\""));
    assert!(markup.contains("data-compact-pane=\"list\""));
    assert!(opening_tag_with_id(&markup, "mobile.people").contains("hidden"));
    assert!(opening_tag_with_id(&markup, "mobile.network").contains("hidden"));
    assert!(opening_tag_with_id(&markup, "mobile.more").contains("hidden"));
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
fn mobile_styles_cover_reflow_safe_areas_targets_and_preferences() {
    for required in [
        "min-inline-size: 20rem",
        "font: -apple-system-body",
        "font: -apple-system-title1",
        "font: -apple-system-caption1",
        "min-block-size: 100dvh",
        "env(safe-area-inset-top)",
        "env(safe-area-inset-bottom)",
        "min-block-size: 2.75rem",
        "[data-target=\"android\"] button",
        "min-block-size: 3rem",
        "@media (min-width: 52rem)",
        "@media (prefers-color-scheme: dark)",
        "@media (prefers-contrast: more)",
        "@media (prefers-reduced-motion: reduce)",
    ] {
        assert!(MOBILE_CSS.contains(required), "missing mobile style contract: {required}");
    }

    assert!(!MOBILE_CSS.contains("outline: none"));
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
    assert!(markup.contains("data-state=\"unavailable\""));
    assert!(markup.contains("id=\"mobile.send\""));
    let send = opening_tag_with_id(&markup, "mobile.send");
    assert!(send.contains("data-enabled=\"false\""));
    assert!(send.contains("disabled"));
}

#[test]
fn network_renders_independent_denied_interrupted_and_unverified_bearers() {
    for (kind, state, reason) in [
        ("bluetooth-rnode", "unavailable", "permission_denied"),
        ("bluetooth-rnode", "disconnected", "connection_interrupted"),
        ("android-usb", "unverified", "physical_evidence_absent"),
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
    let directory = render(fixture("canonical-peer-discovery"));
    let live_empty = render(fixture("live-empty-connected"));

    assert_eq!(directory.matches("id=\"mobile.peer.e01b09b22ccc4e2755d29eead962677b\"").count(), 1);
    assert!(directory.contains("FPIG_SKYWAVE"));
    assert!(!live_empty.contains("id=\"mobile.peer."));
    assert!(!live_empty.contains("FPIG_SKYWAVE"));
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
    assert!(markup.contains("Automatic synchronization enabled"));
    assert!(markup.contains("data-cooldown-secs=\"30\""));
    assert!(markup.contains("data-deadline-secs=\"32\""));
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
    assert!(markup.contains("id=\"mobile.propagation-sync\" disabled"));
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
    let progress_markup = render_propagation(progress);
    assert!(progress_markup.contains("id=\"mobile.propagation-progress\""));
    assert!(progress_markup.contains("data-attempt-id=\"attempt-mobile-sync\""));
    assert!(progress_markup.contains("data-received-count=\"1\""));

    let mut repeated = PropagationUpdate::from_fixture(&completed_fixture);
    repeated.new_messages = 0;
    let repeated_markup = render_propagation(repeated);
    assert!(repeated_markup.contains("id=\"mobile.propagation-result\""));
    assert!(repeated_markup.contains("0 new messages"));

    let failed_fixture = fixture("recoverable-session-failure");
    let mut failed = PropagationUpdate::from_fixture(&failed_fixture);
    failed.failure = Some(TypedFailure { code: "transport_unavailable".into(), retryable: true });
    let failure_markup = render_propagation(failed);
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
    assert!(markup.contains("id=\"mobile.retry.message-direct-1\""));
    assert!(opening_tag_with_id(&markup, "mobile.retry.message-direct-1").contains("disabled"));
}
