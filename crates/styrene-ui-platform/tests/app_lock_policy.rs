//! Deterministic iOS App Lock coverage for `ios-app-lock-policy`.
//!
//! Pure tests are authoritative for the decision matrix, store tests for
//! persistence behavior, and gate tests for startup ordering. None of this is
//! physical `LocalAuthentication` evidence.

use styrene_ui_platform::{
    APP_LOCK_POLICY_KEY, APP_LOCK_SATISFIED_BOOT_KEY, APP_LOCK_SETUP_COMPLETE_KEY,
    AppLockController, AppLockDecision, AppLockEntry, AppLockExemption, AppLockFailure,
    AppLockGateOutcome, AppLockInputs, AppLockPolicy, AppLockStore, BOOT_IDENTITY_TOLERANCE_SECS,
    BootIdentity, DeviceAuthenticationOutcome, DeviceAuthenticator, LaunchIdentity,
    MemoryAppLockStore, app_lock_decision, is_app_lock_failure_code,
};

const LAUNCH_A: LaunchIdentity = LaunchIdentity::new(4001);
const LAUNCH_B: LaunchIdentity = LaunchIdentity::new(4002);
const BOOT_ONE_SECS: i64 = 1_756_700_000;
const BOOT_ONE_SECS_F64: f64 = 1_756_700_000.0;
const BOOT_ONE: BootIdentity = BootIdentity::from_boot_epoch_secs(BOOT_ONE_SECS);
const BOOT_TWO: BootIdentity = BootIdentity::from_boot_epoch_secs(1_756_790_000);
const REASON: &str = "unlock your private mesh communications";
const ALL_POLICIES: [AppLockPolicy; 3] =
    [AppLockPolicy::EveryLaunch, AppLockPolicy::OncePerBoot, AppLockPolicy::Off];

fn inputs(policy: AppLockPolicy, setup_complete: bool) -> AppLockInputs {
    AppLockInputs {
        policy,
        setup_complete,
        launch: LAUNCH_A,
        boot: BOOT_ONE,
        satisfied_launch: None,
        satisfied_boot: None,
    }
}

/// Scripted authenticator that records every request it receives.
struct FakeAuthenticator {
    script: Vec<DeviceAuthenticationOutcome>,
    requests: Vec<String>,
}

impl FakeAuthenticator {
    fn scripted(outcomes: &[DeviceAuthenticationOutcome]) -> Self {
        let mut script = outcomes.to_vec();
        script.reverse();
        Self { script, requests: Vec::new() }
    }

    fn request_count(&self) -> usize {
        self.requests.len()
    }
}

impl DeviceAuthenticator for FakeAuthenticator {
    fn authenticate_device_owner(&mut self, reason: &str) -> DeviceAuthenticationOutcome {
        self.requests.push(reason.to_owned());
        self.script.pop().expect("authenticator was asked more often than the test scripted")
    }
}

/// Backend-start spy: the private session starts only when the gate opened.
struct BackendStartSpy {
    starts: usize,
}

impl BackendStartSpy {
    fn start_if_opened(&mut self, outcome: AppLockGateOutcome) -> AppLockGateOutcome {
        if outcome.opened() {
            self.starts += 1;
        }
        outcome
    }
}

fn completed_controller(policy: AppLockPolicy) -> AppLockController<MemoryAppLockStore> {
    let mut controller = AppLockController::new(MemoryAppLockStore::default());
    controller.set_policy(policy);
    controller.record_setup_complete();
    controller
}

// ── 1. Pure policy decisions ────────────────────────────────────────────────

#[test]
fn persisted_policy_values_resolve_with_a_locked_fallback() {
    let table: [(Option<&str>, AppLockPolicy); 10] = [
        (None, AppLockPolicy::EveryLaunch),
        (Some(""), AppLockPolicy::EveryLaunch),
        (Some("every_launch"), AppLockPolicy::EveryLaunch),
        (Some("once_per_boot"), AppLockPolicy::OncePerBoot),
        (Some("off"), AppLockPolicy::Off),
        (Some(" off \n"), AppLockPolicy::Off),
        (Some("Off"), AppLockPolicy::EveryLaunch),
        (Some("OFF"), AppLockPolicy::EveryLaunch),
        (Some("never"), AppLockPolicy::EveryLaunch),
        (Some("once_per_boot;off"), AppLockPolicy::EveryLaunch),
    ];

    for (persisted, expected) in table {
        assert_eq!(AppLockPolicy::resolve_persisted(persisted), expected, "{persisted:?}");
    }
    assert_eq!(AppLockPolicy::default(), AppLockPolicy::EveryLaunch);
    for policy in ALL_POLICIES {
        assert_eq!(AppLockPolicy::resolve_persisted(Some(policy.as_str())), policy);
    }
}

#[test]
fn incomplete_setup_exempts_every_policy_and_every_satisfaction_state() {
    for policy in ALL_POLICIES {
        for satisfied_launch in [None, Some(LAUNCH_A), Some(LAUNCH_B)] {
            for satisfied_boot in [None, Some(BOOT_ONE), Some(BOOT_TWO)] {
                let decision = app_lock_decision(AppLockInputs {
                    satisfied_launch,
                    satisfied_boot,
                    ..inputs(policy, false)
                });
                assert_eq!(
                    decision,
                    AppLockDecision::NotRequired(AppLockExemption::SetupIncomplete),
                    "{policy:?} {satisfied_launch:?} {satisfied_boot:?}"
                );
            }
        }
    }
}

#[test]
fn decision_matrix_covers_every_policy_launch_boot_and_prior_satisfaction() {
    use AppLockDecision::{NotRequired, Required};
    use AppLockExemption::{PolicyOff, SatisfiedThisBoot, SatisfiedThisLaunch};

    let matrix: [(AppLockPolicy, Option<LaunchIdentity>, Option<BootIdentity>, AppLockDecision);
        12] = [
        (AppLockPolicy::EveryLaunch, None, None, Required),
        (AppLockPolicy::EveryLaunch, Some(LAUNCH_A), None, NotRequired(SatisfiedThisLaunch)),
        (AppLockPolicy::EveryLaunch, Some(LAUNCH_B), None, Required),
        (AppLockPolicy::EveryLaunch, None, Some(BOOT_ONE), Required),
        (AppLockPolicy::EveryLaunch, Some(LAUNCH_B), Some(BOOT_ONE), Required),
        (AppLockPolicy::OncePerBoot, None, None, Required),
        (AppLockPolicy::OncePerBoot, None, Some(BOOT_ONE), NotRequired(SatisfiedThisBoot)),
        (AppLockPolicy::OncePerBoot, None, Some(BOOT_TWO), Required),
        (AppLockPolicy::OncePerBoot, Some(LAUNCH_A), None, Required),
        (
            AppLockPolicy::OncePerBoot,
            Some(LAUNCH_B),
            Some(BOOT_ONE),
            NotRequired(SatisfiedThisBoot),
        ),
        (AppLockPolicy::Off, None, None, NotRequired(PolicyOff)),
        (AppLockPolicy::Off, Some(LAUNCH_B), Some(BOOT_TWO), NotRequired(PolicyOff)),
    ];

    for (policy, satisfied_launch, satisfied_boot, expected) in matrix {
        let decision = app_lock_decision(AppLockInputs {
            satisfied_launch,
            satisfied_boot,
            ..inputs(policy, true)
        });
        assert_eq!(decision, expected, "{policy:?} {satisfied_launch:?} {satisfied_boot:?}");
        assert_eq!(decision.requires_authentication(), expected == Required);
    }
}

#[test]
fn boot_identity_tolerates_sampling_drift_but_not_a_new_boot() {
    let base = BOOT_ONE.boot_epoch_secs();
    for drift in -BOOT_IDENTITY_TOLERANCE_SECS..=BOOT_IDENTITY_TOLERANCE_SECS {
        assert!(BOOT_ONE.same_boot(BootIdentity::from_boot_epoch_secs(base + drift)), "{drift}");
    }
    for beyond in [BOOT_IDENTITY_TOLERANCE_SECS + 1, -(BOOT_IDENTITY_TOLERANCE_SECS + 1), 86_400] {
        assert!(!BOOT_ONE.same_boot(BootIdentity::from_boot_epoch_secs(base + beyond)), "{beyond}");
    }
    assert!(!BOOT_ONE.same_boot(BOOT_TWO));
}

#[test]
fn every_launch_backend_retry_does_not_duplicate_the_request() {
    let mut controller = completed_controller(AppLockPolicy::EveryLaunch);
    let mut authenticator =
        FakeAuthenticator::scripted(&[DeviceAuthenticationOutcome::Authenticated]);

    let first = controller.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON);
    let retry = controller.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON);
    let next_process = controller.decision(LAUNCH_B, BOOT_ONE);

    assert_eq!(first, AppLockGateOutcome::Opened(AppLockEntry::Authenticated));
    assert_eq!(
        retry,
        AppLockGateOutcome::Opened(AppLockEntry::Exempt(AppLockExemption::SatisfiedThisLaunch))
    );
    assert_eq!(authenticator.request_count(), 1, "same-process retry prompted again");
    assert_eq!(next_process, AppLockDecision::Required, "a new process inherited launch state");
}

#[test]
fn once_per_boot_relaunch_is_exempt_until_the_device_reboots() {
    let mut controller = completed_controller(AppLockPolicy::OncePerBoot);
    let mut authenticator =
        FakeAuthenticator::scripted(&[DeviceAuthenticationOutcome::Authenticated]);

    let first = controller.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON);
    assert_eq!(first, AppLockGateOutcome::Opened(AppLockEntry::Authenticated));

    // A cold relaunch in the same boot is a new launch identity with the boot
    // marker sampled a little later.
    let relaunch = controller
        .decision(LAUNCH_B, BootIdentity::from_boot_epoch_secs(BOOT_ONE.boot_epoch_secs() + 2));
    assert_eq!(relaunch, AppLockDecision::NotRequired(AppLockExemption::SatisfiedThisBoot));

    let after_reboot = controller.decision(LAUNCH_B, BOOT_TWO);
    assert_eq!(after_reboot, AppLockDecision::Required, "reboot did not invalidate satisfaction");
    assert_eq!(authenticator.request_count(), 1);
}

// ── 2. Authentication and startup ordering ──────────────────────────────────

#[test]
fn only_an_authenticated_outcome_starts_a_required_private_session() {
    let closed_outcomes = [
        (DeviceAuthenticationOutcome::Cancelled, "app_unlock_cancelled"),
        (DeviceAuthenticationOutcome::Unavailable, "app_unlock_unavailable"),
        (DeviceAuthenticationOutcome::Failed, "app_unlock_failed"),
    ];

    for (outcome, code) in closed_outcomes {
        let mut controller = completed_controller(AppLockPolicy::EveryLaunch);
        let mut authenticator = FakeAuthenticator::scripted(&[outcome]);
        let mut backend = BackendStartSpy { starts: 0 };

        let result = backend.start_if_opened(controller.gate(
            LAUNCH_A,
            BOOT_ONE,
            &mut authenticator,
            REASON,
        ));

        let failure = AppLockFailure { outcome };
        assert_eq!(result, AppLockGateOutcome::Closed(failure), "{outcome:?}");
        assert_eq!(failure.code(), code);
        assert!(failure.retryable());
        assert!(is_app_lock_failure_code(code));
        assert_eq!(backend.starts, 0, "{outcome:?} started the private session");
        assert_eq!(controller.store().satisfied_launch, None, "{outcome:?} recorded launch");
        assert_eq!(controller.store().satisfied_boot_epoch_secs, None, "{outcome:?} recorded boot");
        assert_eq!(controller.decision(LAUNCH_A, BOOT_ONE), AppLockDecision::Required);
    }

    let mut controller = completed_controller(AppLockPolicy::EveryLaunch);
    let mut authenticator =
        FakeAuthenticator::scripted(&[DeviceAuthenticationOutcome::Authenticated]);
    let mut backend = BackendStartSpy { starts: 0 };
    let result =
        backend.start_if_opened(controller.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON));
    assert_eq!(result, AppLockGateOutcome::Opened(AppLockEntry::Authenticated));
    assert_eq!(backend.starts, 1);
    assert_eq!(authenticator.requests, [REASON]);
    assert!(!is_app_lock_failure_code("embedded_start_failed"));
}

#[test]
fn explicit_retry_after_a_closed_outcome_requests_authentication_again() {
    let mut controller = completed_controller(AppLockPolicy::EveryLaunch);
    let mut authenticator = FakeAuthenticator::scripted(&[
        DeviceAuthenticationOutcome::Cancelled,
        DeviceAuthenticationOutcome::Failed,
        DeviceAuthenticationOutcome::Authenticated,
    ]);
    let mut backend = BackendStartSpy { starts: 0 };

    let outcomes: Vec<AppLockGateOutcome> = (0..3)
        .map(|_| {
            backend.start_if_opened(controller.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON))
        })
        .collect();

    assert!(matches!(outcomes[0], AppLockGateOutcome::Closed(_)));
    assert!(matches!(outcomes[1], AppLockGateOutcome::Closed(_)));
    assert_eq!(outcomes[2], AppLockGateOutcome::Opened(AppLockEntry::Authenticated));
    assert_eq!(authenticator.request_count(), 3);
    assert_eq!(backend.starts, 1);
}

#[test]
fn setup_and_migration_paths_issue_no_app_lock_request() {
    for policy in ALL_POLICIES {
        // Fresh install: nothing persisted at all.
        let mut fresh = AppLockController::new(MemoryAppLockStore::default());
        let mut authenticator = FakeAuthenticator::scripted(&[]);
        let mut backend = BackendStartSpy { starts: 0 };
        let outcome =
            backend.start_if_opened(fresh.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON));
        assert_eq!(
            outcome,
            AppLockGateOutcome::Opened(AppLockEntry::Exempt(AppLockExemption::SetupIncomplete))
        );
        assert_eq!(authenticator.request_count(), 0, "{policy:?} fresh setup prompted");
        assert_eq!(backend.starts, 1);

        // Legacy custody migration: a policy was chosen but setup never completed.
        let mut migrating = AppLockController::new(MemoryAppLockStore::default());
        migrating.set_policy(policy);
        let outcome = migrating.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON);
        assert_eq!(
            outcome,
            AppLockGateOutcome::Opened(AppLockEntry::Exempt(AppLockExemption::SetupIncomplete))
        );
        assert_eq!(authenticator.request_count(), 0, "{policy:?} migration prompted");
        assert_eq!(migrating.store().satisfied_launch, None);
        assert_eq!(migrating.store().satisfied_boot_epoch_secs, None);
    }
}

#[test]
fn satisfaction_is_recorded_only_after_its_successful_event() {
    let mut controller = AppLockController::new(MemoryAppLockStore::default());
    controller.set_policy(AppLockPolicy::OncePerBoot);
    assert!(!controller.setup_complete());

    // Setup that fails to produce a usable session records nothing.
    let mut authenticator = FakeAuthenticator::scripted(&[]);
    let gate = controller.gate(LAUNCH_A, BOOT_ONE, &mut authenticator, REASON);
    assert!(gate.opened());
    assert_eq!(controller.store().setup_complete, None);
    assert_eq!(controller.store().satisfied_boot_epoch_secs, None);

    // Setup succeeds: the owner records completion after the session exists.
    controller.record_setup_complete();
    assert!(controller.setup_complete());
    assert_eq!(controller.decision(LAUNCH_B, BOOT_ONE), AppLockDecision::Required);

    // Authentication success followed by backend failure keeps satisfaction.
    let mut authenticator =
        FakeAuthenticator::scripted(&[DeviceAuthenticationOutcome::Authenticated]);
    let gate = controller.gate(LAUNCH_B, BOOT_ONE, &mut authenticator, REASON);
    assert_eq!(gate, AppLockGateOutcome::Opened(AppLockEntry::Authenticated));
    assert_eq!(controller.store().satisfied_launch, Some(LAUNCH_B.value()));
    assert_eq!(controller.store().satisfied_boot_epoch_secs, Some(BOOT_ONE_SECS_F64));
    let backend_retry = controller.gate(LAUNCH_B, BOOT_ONE, &mut authenticator, REASON);
    assert_eq!(
        backend_retry,
        AppLockGateOutcome::Opened(AppLockEntry::Exempt(AppLockExemption::SatisfiedThisBoot))
    );
    assert_eq!(authenticator.request_count(), 1);
}

// ── 3. Persistence adapters ─────────────────────────────────────────────────

#[test]
fn isolated_store_round_trips_policy_setup_and_satisfaction_independently() {
    let mut controller = AppLockController::new(MemoryAppLockStore::default());
    assert_eq!(controller.store(), &MemoryAppLockStore::default());

    controller.set_policy(AppLockPolicy::OncePerBoot);
    assert_eq!(controller.store().policy.as_deref(), Some("once_per_boot"));
    assert_eq!(controller.policy(), AppLockPolicy::OncePerBoot);
    assert_eq!(controller.store().setup_complete, None, "policy write touched setup");
    assert_eq!(controller.store().satisfied_boot_epoch_secs, None, "policy write touched boot");

    controller.record_setup_complete();
    assert_eq!(controller.store().setup_complete, Some(true));
    assert_eq!(controller.store().policy.as_deref(), Some("once_per_boot"));

    controller.record_authentication(LAUNCH_A, BOOT_ONE);
    assert_eq!(controller.store().satisfied_launch, Some(LAUNCH_A.value()));
    assert_eq!(controller.store().satisfied_boot_epoch_secs, Some(BOOT_ONE_SECS_F64));

    controller.set_policy(AppLockPolicy::Off);
    assert_eq!(controller.policy(), AppLockPolicy::Off);
    assert_eq!(controller.store().setup_complete, Some(true));
    assert_eq!(controller.store().satisfied_launch, Some(LAUNCH_A.value()));
}

#[test]
fn absent_and_malformed_store_values_fail_closed() {
    let absent = AppLockController::new(MemoryAppLockStore::default());
    assert_eq!(absent.policy(), AppLockPolicy::EveryLaunch);
    assert!(!absent.setup_complete());
    let inputs = absent.inputs(LAUNCH_A, BOOT_ONE);
    assert_eq!(inputs.satisfied_launch, None);
    assert_eq!(inputs.satisfied_boot, None);

    let malformed = AppLockController::new(MemoryAppLockStore {
        policy: Some("disabled".into()),
        setup_complete: Some(true),
        satisfied_boot_epoch_secs: Some(f64::NAN),
        satisfied_launch: None,
    });
    assert_eq!(malformed.policy(), AppLockPolicy::EveryLaunch);
    assert_eq!(malformed.inputs(LAUNCH_A, BOOT_ONE).satisfied_boot, None);
    assert_eq!(malformed.decision(LAUNCH_A, BOOT_ONE), AppLockDecision::Required);

    let infinite = AppLockController::new(MemoryAppLockStore {
        policy: Some("once_per_boot".into()),
        setup_complete: Some(true),
        satisfied_boot_epoch_secs: Some(f64::INFINITY),
        satisfied_launch: None,
    });
    assert_eq!(infinite.decision(LAUNCH_A, BOOT_ONE), AppLockDecision::Required);

    let explicit_false = AppLockController::new(MemoryAppLockStore {
        policy: Some("off".into()),
        setup_complete: Some(false),
        satisfied_boot_epoch_secs: None,
        satisfied_launch: None,
    });
    assert_eq!(
        explicit_false.decision(LAUNCH_A, BOOT_ONE),
        AppLockDecision::NotRequired(AppLockExemption::SetupIncomplete)
    );
}

#[test]
fn boot_identity_and_clock_changes_are_handled_deterministically() {
    let mut controller = completed_controller(AppLockPolicy::OncePerBoot);
    let mut authenticator =
        FakeAuthenticator::scripted(&[DeviceAuthenticationOutcome::Authenticated]);
    let sampled = BootIdentity::from_boot_epoch_secs(BOOT_ONE.boot_epoch_secs());
    assert!(controller.gate(LAUNCH_A, sampled, &mut authenticator, REASON).opened());

    // Fractional persisted markers round to the nearest second.
    let mut fractional = MemoryAppLockStore::default();
    fractional.set_policy("once_per_boot");
    fractional.set_setup_complete(true);
    fractional.set_satisfied_boot_epoch_secs(BOOT_ONE_SECS_F64 + 0.49);
    let fractional = AppLockController::new(fractional);
    assert_eq!(fractional.inputs(LAUNCH_B, BOOT_ONE).satisfied_boot, Some(BOOT_ONE));

    // Sampling drift within tolerance stays satisfied.
    let drifted = BootIdentity::from_boot_epoch_secs(BOOT_ONE.boot_epoch_secs() + 3);
    assert_eq!(
        controller.decision(LAUNCH_B, drifted),
        AppLockDecision::NotRequired(AppLockExemption::SatisfiedThisBoot)
    );

    // A wall-clock change beyond tolerance reads as a new boot and fails closed.
    let clock_jump = BootIdentity::from_boot_epoch_secs(BOOT_ONE.boot_epoch_secs() + 3_600);
    assert_eq!(controller.decision(LAUNCH_B, clock_jump), AppLockDecision::Required);
    let clock_back = BootIdentity::from_boot_epoch_secs(BOOT_ONE.boot_epoch_secs() - 3_600);
    assert_eq!(controller.decision(LAUNCH_B, clock_back), AppLockDecision::Required);
}

#[test]
fn persistence_keys_are_stable_and_carry_no_identity_material() {
    assert_eq!(APP_LOCK_POLICY_KEY, "io.styrene.app-lock.policy");
    assert_eq!(APP_LOCK_SETUP_COMPLETE_KEY, "io.styrene.app-lock.setup-complete");
    assert_eq!(APP_LOCK_SATISFIED_BOOT_KEY, "io.styrene.app-lock.authenticated-boot");
    for key in [APP_LOCK_POLICY_KEY, APP_LOCK_SETUP_COMPLETE_KEY, APP_LOCK_SATISFIED_BOOT_KEY] {
        assert!(!key.contains("identity"));
        assert!(!key.contains("keychain"));
    }

    let mut controller = completed_controller(AppLockPolicy::EveryLaunch);
    controller.record_authentication(LAUNCH_A, BOOT_ONE);
    let persisted = format!("{:?}", controller.store());
    assert!(!persisted.contains("keychain"));
    assert!(!persisted.contains("custody"));
}
