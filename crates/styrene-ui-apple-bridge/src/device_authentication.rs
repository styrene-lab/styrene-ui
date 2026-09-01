//! iOS adapters for the pure App Lock policy in `styrene-ui-platform`.
//!
//! This module only acquires Apple state. Policy decisions, satisfaction
//! recording, and startup ordering live in `AppLockController`, where they are
//! tested without Apple frameworks.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use block2::RcBlock;
use objc2::{AnyThread, runtime::Bool};
use objc2_foundation::{NSDate, NSError, NSProcessInfo, NSString, NSUserDefaults};
use objc2_local_authentication::{LAContext, LAError, LAPolicy};
use styrene_ui_platform::{
    APP_LOCK_POLICY_KEY, APP_LOCK_SATISFIED_BOOT_KEY, APP_LOCK_SETUP_COMPLETE_KEY,
    AppLockController, AppLockGateOutcome, AppLockPolicy, AppLockStore, BootIdentity,
    DeviceAuthenticationOutcome, DeviceAuthenticator, LaunchIdentity,
};

/// Launch satisfaction is process-scoped and intentionally never persisted.
/// Zero means no launch has been satisfied in this process.
static SATISFIED_LAUNCH: AtomicU64 = AtomicU64::new(0);

/// Bounded `NSUserDefaults` adapter for durable App Lock state.
///
/// Absent keys read as `None` so the controller fails closed. No identity
/// material or custody result is ever written through this adapter.
#[derive(Clone, Copy, Debug, Default)]
pub struct UserDefaultsAppLockStore;

impl UserDefaultsAppLockStore {
    fn has_value(key: &NSString) -> bool {
        NSUserDefaults::standardUserDefaults().objectForKey(key).is_some()
    }
}

impl AppLockStore for UserDefaultsAppLockStore {
    fn policy(&self) -> Option<String> {
        let key = NSString::from_str(APP_LOCK_POLICY_KEY);
        NSUserDefaults::standardUserDefaults().stringForKey(&key).map(|value| value.to_string())
    }

    fn set_policy(&mut self, value: &str) {
        let defaults = NSUserDefaults::standardUserDefaults();
        let key = NSString::from_str(APP_LOCK_POLICY_KEY);
        let value = NSString::from_str(value);
        // SAFETY: NSString is a supported NSUserDefaults property-list value.
        unsafe { defaults.setObject_forKey(Some(&value), &key) };
    }

    fn setup_complete(&self) -> Option<bool> {
        let key = NSString::from_str(APP_LOCK_SETUP_COMPLETE_KEY);
        Self::has_value(&key).then(|| NSUserDefaults::standardUserDefaults().boolForKey(&key))
    }

    fn set_setup_complete(&mut self, complete: bool) {
        NSUserDefaults::standardUserDefaults()
            .setBool_forKey(complete, &NSString::from_str(APP_LOCK_SETUP_COMPLETE_KEY));
    }

    fn satisfied_boot_epoch_secs(&self) -> Option<f64> {
        let key = NSString::from_str(APP_LOCK_SATISFIED_BOOT_KEY);
        Self::has_value(&key).then(|| NSUserDefaults::standardUserDefaults().doubleForKey(&key))
    }

    fn set_satisfied_boot_epoch_secs(&mut self, boot_epoch_secs: f64) {
        NSUserDefaults::standardUserDefaults()
            .setDouble_forKey(boot_epoch_secs, &NSString::from_str(APP_LOCK_SATISFIED_BOOT_KEY));
    }

    fn satisfied_launch(&self) -> Option<u64> {
        match SATISFIED_LAUNCH.load(Ordering::Acquire) {
            0 => None,
            launch => Some(launch),
        }
    }

    fn set_satisfied_launch(&mut self, launch: u64) {
        SATISFIED_LAUNCH.store(launch, Ordering::Release);
    }
}

/// LocalAuthentication adapter. One request per call; the native context is
/// retained until the reply arrives.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalAuthenticationAuthenticator;

impl DeviceAuthenticator for LocalAuthenticationAuthenticator {
    fn authenticate_device_owner(&mut self, reason: &str) -> DeviceAuthenticationOutcome {
        authenticate_device_owner(reason)
    }
}

#[must_use]
pub fn app_lock_controller() -> AppLockController<UserDefaultsAppLockStore> {
    AppLockController::new(UserDefaultsAppLockStore)
}

/// The current process. Process identifiers are never zero on iOS, and zero is
/// reserved in the store for "no satisfied launch".
#[must_use]
pub fn current_launch_identity() -> LaunchIdentity {
    LaunchIdentity::new(u64::from(std::process::id()).max(1))
}

/// Approximate boot instant derived from wall-clock time minus system uptime.
#[must_use]
pub fn current_boot_identity() -> BootIdentity {
    let marker =
        NSDate::date().timeIntervalSince1970() - NSProcessInfo::processInfo().systemUptime();
    BootIdentity::from_boot_epoch_secs(marker.round() as i64)
}

#[must_use]
pub fn app_lock_policy() -> AppLockPolicy {
    app_lock_controller().policy()
}

pub fn store_app_lock_policy(policy: AppLockPolicy) {
    app_lock_controller().set_policy(policy);
}

/// Report whether the next gate evaluation will request authentication.
#[must_use]
pub fn app_lock_requires_authentication() -> bool {
    app_lock_controller()
        .decision(current_launch_identity(), current_boot_identity())
        .requires_authentication()
}

/// Gate private session startup against the persisted policy for this launch
/// and boot. Satisfaction is recorded only after an `Authenticated` outcome.
pub fn gate_app_lock<A: DeviceAuthenticator>(
    authenticator: &mut A,
    reason: &str,
) -> AppLockGateOutcome {
    app_lock_controller().gate(
        current_launch_identity(),
        current_boot_identity(),
        authenticator,
        reason,
    )
}

/// Record setup completion. Call only after a usable backend session exists.
pub fn record_app_lock_setup_complete() {
    app_lock_controller().record_setup_complete();
}

/// Authenticate the device owner once while retaining the native context until completion.
pub fn authenticate_device_owner(reason: &str) -> DeviceAuthenticationOutcome {
    let context = unsafe { LAContext::init(LAContext::alloc()) };
    let policy = LAPolicy::DeviceOwnerAuthentication;
    if let Err(error) = unsafe { context.canEvaluatePolicy_error(policy) } {
        return map_error(&error);
    }

    let reason = NSString::from_str(reason);
    let (sender, receiver) = mpsc::sync_channel(1);
    let reply = RcBlock::new(move |success: Bool, error: *mut NSError| {
        let outcome = if success.as_bool() {
            DeviceAuthenticationOutcome::Authenticated
        } else if error.is_null() {
            DeviceAuthenticationOutcome::Failed
        } else {
            // SAFETY: LocalAuthentication guarantees that a non-null error pointer
            // is valid for the duration of this reply block.
            map_error(unsafe { &*error })
        };
        let _ = sender.try_send(outcome);
    });

    // SAFETY: The reason is non-empty, the escaping block is heap-backed and
    // sendable, and `context` remains retained until the reply is received.
    unsafe { context.evaluatePolicy_localizedReason_reply(policy, &reason, &reply) };
    receiver.recv().unwrap_or(DeviceAuthenticationOutcome::Failed)
}

fn map_error(error: &NSError) -> DeviceAuthenticationOutcome {
    let code = error.code();
    if code == LAError::UserCancel.0
        || code == LAError::SystemCancel.0
        || code == LAError::AppCancel.0
    {
        DeviceAuthenticationOutcome::Cancelled
    } else if code == LAError::PasscodeNotSet.0
        || code == LAError::BiometryNotAvailable.0
        || code == LAError::BiometryNotEnrolled.0
    {
        DeviceAuthenticationOutcome::Unavailable
    } else {
        DeviceAuthenticationOutcome::Failed
    }
}
