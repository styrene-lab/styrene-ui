//! Pure iOS App Lock policy decisions.
//!
//! App Lock controls entry to the application session. Identity custody is a
//! separate boundary and never appears in these inputs. Apple adapters obtain
//! launch and boot identity, persist state, and invoke `LocalAuthentication`.
//! Everything decided here is deterministic and testable without Apple
//! frameworks.

use crate::{AppLockPolicy, DeviceAuthenticationOutcome};

/// Boot identities within this many seconds describe the same device boot.
///
/// The Apple adapter derives boot identity from wall-clock time minus system
/// uptime. Both values are sampled separately, so a small drift is expected.
pub const BOOT_IDENTITY_TOLERANCE_SECS: i64 = 5;

pub const APP_LOCK_POLICY_KEY: &str = "io.styrene.app-lock.policy";
pub const APP_LOCK_SETUP_COMPLETE_KEY: &str = "io.styrene.app-lock.setup-complete";
pub const APP_LOCK_SATISFIED_BOOT_KEY: &str = "io.styrene.app-lock.authenticated-boot";

impl AppLockPolicy {
    /// Resolve a persisted value. Absent and unrecognized values fall back to
    /// `EveryLaunch` so a corrupt store can never weaken the lock to `Off`.
    #[must_use]
    pub fn resolve_persisted(value: Option<&str>) -> Self {
        value.map(str::trim).and_then(Self::parse).unwrap_or_default()
    }
}

/// Identifies one application process.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LaunchIdentity(u64);

impl LaunchIdentity {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Identifies one device boot by its approximate boot instant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootIdentity {
    boot_epoch_secs: i64,
}

impl BootIdentity {
    #[must_use]
    pub const fn from_boot_epoch_secs(boot_epoch_secs: i64) -> Self {
        Self { boot_epoch_secs }
    }

    #[must_use]
    pub const fn boot_epoch_secs(self) -> i64 {
        self.boot_epoch_secs
    }

    /// Two samples describe the same boot when they agree within tolerance.
    /// A wall-clock change larger than the tolerance reads as a new boot and
    /// fails closed by requiring authentication again.
    #[must_use]
    pub const fn same_boot(self, other: Self) -> bool {
        self.boot_epoch_secs.abs_diff(other.boot_epoch_secs) <= BOOT_IDENTITY_TOLERANCE_SECS as u64
    }
}

/// Every fact the decision needs. Nothing here is an identity secret.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppLockInputs {
    pub policy: AppLockPolicy,
    pub setup_complete: bool,
    pub launch: LaunchIdentity,
    pub boot: BootIdentity,
    pub satisfied_launch: Option<LaunchIdentity>,
    pub satisfied_boot: Option<BootIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLockExemption {
    SetupIncomplete,
    PolicyOff,
    SatisfiedThisLaunch,
    SatisfiedThisBoot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLockDecision {
    Required,
    NotRequired(AppLockExemption),
}

impl AppLockDecision {
    #[must_use]
    pub const fn requires_authentication(self) -> bool {
        matches!(self, Self::Required)
    }
}

/// Decide whether App Lock must request device-owner authentication.
#[must_use]
pub const fn app_lock_decision(inputs: AppLockInputs) -> AppLockDecision {
    if !inputs.setup_complete {
        return AppLockDecision::NotRequired(AppLockExemption::SetupIncomplete);
    }
    match inputs.policy {
        AppLockPolicy::Off => AppLockDecision::NotRequired(AppLockExemption::PolicyOff),
        AppLockPolicy::EveryLaunch => match inputs.satisfied_launch {
            Some(satisfied) if satisfied.0 == inputs.launch.0 => {
                AppLockDecision::NotRequired(AppLockExemption::SatisfiedThisLaunch)
            }
            _ => AppLockDecision::Required,
        },
        AppLockPolicy::OncePerBoot => match inputs.satisfied_boot {
            Some(satisfied) if satisfied.same_boot(inputs.boot) => {
                AppLockDecision::NotRequired(AppLockExemption::SatisfiedThisBoot)
            }
            _ => AppLockDecision::Required,
        },
    }
}

/// Bounded persistence for App Lock state.
///
/// Policy, setup completion, and boot satisfaction survive process restarts.
/// Launch satisfaction is process-scoped and must never be written to durable
/// storage, or a later cold launch would inherit it. Readers return `None` for
/// absent or malformed values and let the controller fail closed.
pub trait AppLockStore {
    fn policy(&self) -> Option<String>;
    fn set_policy(&mut self, value: &str);
    fn setup_complete(&self) -> Option<bool>;
    fn set_setup_complete(&mut self, complete: bool);
    fn satisfied_boot_epoch_secs(&self) -> Option<f64>;
    fn set_satisfied_boot_epoch_secs(&mut self, boot_epoch_secs: f64);
    fn satisfied_launch(&self) -> Option<u64>;
    fn set_satisfied_launch(&mut self, launch: u64);
}

/// Deterministic in-memory store for tests and fixtures.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemoryAppLockStore {
    pub policy: Option<String>,
    pub setup_complete: Option<bool>,
    pub satisfied_boot_epoch_secs: Option<f64>,
    pub satisfied_launch: Option<u64>,
}

impl AppLockStore for MemoryAppLockStore {
    fn policy(&self) -> Option<String> {
        self.policy.clone()
    }

    fn set_policy(&mut self, value: &str) {
        self.policy = Some(value.to_owned());
    }

    fn setup_complete(&self) -> Option<bool> {
        self.setup_complete
    }

    fn set_setup_complete(&mut self, complete: bool) {
        self.setup_complete = Some(complete);
    }

    fn satisfied_boot_epoch_secs(&self) -> Option<f64> {
        self.satisfied_boot_epoch_secs
    }

    fn set_satisfied_boot_epoch_secs(&mut self, boot_epoch_secs: f64) {
        self.satisfied_boot_epoch_secs = Some(boot_epoch_secs);
    }

    fn satisfied_launch(&self) -> Option<u64> {
        self.satisfied_launch
    }

    fn set_satisfied_launch(&mut self, launch: u64) {
        self.satisfied_launch = Some(launch);
    }
}

/// Requests device-owner authentication. The Apple adapter wraps
/// `LocalAuthentication`; tests substitute a scripted authenticator.
pub trait DeviceAuthenticator {
    fn authenticate_device_owner(&mut self, reason: &str) -> DeviceAuthenticationOutcome;
}

/// Typed closed outcome of an App Lock request. Every variant permits retry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppLockFailure {
    pub outcome: DeviceAuthenticationOutcome,
}

impl AppLockFailure {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self.outcome {
            DeviceAuthenticationOutcome::Cancelled => "app_unlock_cancelled",
            DeviceAuthenticationOutcome::Unavailable => "app_unlock_unavailable",
            DeviceAuthenticationOutcome::Failed | DeviceAuthenticationOutcome::Authenticated => {
                "app_unlock_failed"
            }
        }
    }

    #[must_use]
    pub const fn retryable(self) -> bool {
        true
    }
}

/// Reports whether a failure code belongs to App Lock so presentation can
/// offer an explicit retry without inspecting other failure families.
#[must_use]
pub fn is_app_lock_failure_code(code: &str) -> bool {
    matches!(code, "app_unlock_cancelled" | "app_unlock_unavailable" | "app_unlock_failed")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLockEntry {
    Exempt(AppLockExemption),
    Authenticated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppLockGateOutcome {
    Opened(AppLockEntry),
    Closed(AppLockFailure),
}

impl AppLockGateOutcome {
    #[must_use]
    pub const fn opened(self) -> bool {
        matches!(self, Self::Opened(_))
    }
}

/// Owns the policy inputs behind a bounded store and records state only after
/// the event it represents has succeeded.
#[derive(Debug)]
pub struct AppLockController<S> {
    store: S,
}

impl<S: AppLockStore> AppLockController<S> {
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }

    #[must_use]
    pub fn policy(&self) -> AppLockPolicy {
        AppLockPolicy::resolve_persisted(self.store.policy().as_deref())
    }

    /// Persist the exact selected policy. Identity custody is untouched.
    pub fn set_policy(&mut self, policy: AppLockPolicy) {
        self.store.set_policy(policy.as_str());
    }

    #[must_use]
    pub fn setup_complete(&self) -> bool {
        self.store.setup_complete().unwrap_or(false)
    }

    /// Record setup completion. Call only after a usable backend session exists.
    pub fn record_setup_complete(&mut self) {
        self.store.set_setup_complete(true);
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "boot markers are Unix seconds rounded from a finite double; they fit in i64"
    )]
    pub fn inputs(&self, launch: LaunchIdentity, boot: BootIdentity) -> AppLockInputs {
        AppLockInputs {
            policy: self.policy(),
            setup_complete: self.setup_complete(),
            launch,
            boot,
            satisfied_launch: self.store.satisfied_launch().map(LaunchIdentity::new),
            satisfied_boot: self
                .store
                .satisfied_boot_epoch_secs()
                .filter(|secs| secs.is_finite())
                .map(|secs| BootIdentity::from_boot_epoch_secs(secs.round() as i64)),
        }
    }

    #[must_use]
    pub fn decision(&self, launch: LaunchIdentity, boot: BootIdentity) -> AppLockDecision {
        app_lock_decision(self.inputs(launch, boot))
    }

    /// Record a successful device-owner authentication for this launch and boot.
    #[expect(
        clippy::cast_precision_loss,
        reason = "boot markers are Unix seconds, far below the f64 exact-integer limit"
    )]
    pub fn record_authentication(&mut self, launch: LaunchIdentity, boot: BootIdentity) {
        self.store.set_satisfied_launch(launch.value());
        self.store.set_satisfied_boot_epoch_secs(boot.boot_epoch_secs() as f64);
    }

    /// Gate private session startup. Authentication is requested only when the
    /// decision requires it, and satisfaction is recorded only after an
    /// `Authenticated` outcome. Callers start the private backend only when the
    /// result is `Opened`.
    pub fn gate<A: DeviceAuthenticator>(
        &mut self,
        launch: LaunchIdentity,
        boot: BootIdentity,
        authenticator: &mut A,
        reason: &str,
    ) -> AppLockGateOutcome {
        match self.decision(launch, boot) {
            AppLockDecision::NotRequired(exemption) => {
                AppLockGateOutcome::Opened(AppLockEntry::Exempt(exemption))
            }
            AppLockDecision::Required => {
                let outcome = authenticator.authenticate_device_owner(reason);
                if outcome == DeviceAuthenticationOutcome::Authenticated {
                    self.record_authentication(launch, boot);
                    AppLockGateOutcome::Opened(AppLockEntry::Authenticated)
                } else {
                    AppLockGateOutcome::Closed(AppLockFailure { outcome })
                }
            }
        }
    }
}
