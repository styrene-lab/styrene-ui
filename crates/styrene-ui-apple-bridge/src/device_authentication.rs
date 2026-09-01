use std::sync::mpsc;

use block2::RcBlock;
use objc2::{AnyThread, runtime::Bool};
use objc2_foundation::{NSDate, NSError, NSProcessInfo, NSString, NSUserDefaults};
use objc2_local_authentication::{LAContext, LAError, LAPolicy};
use styrene_ui_platform::{AppLockPolicy, DeviceAuthenticationOutcome};

const POLICY_KEY: &str = "io.styrene.app-lock.policy";
const SETUP_COMPLETE_KEY: &str = "io.styrene.app-lock.setup-complete";
const AUTHENTICATED_BOOT_KEY: &str = "io.styrene.app-lock.authenticated-boot";
const BOOT_MARKER_TOLERANCE_SECS: f64 = 5.0;

#[must_use]
pub fn app_lock_policy() -> AppLockPolicy {
    let key = NSString::from_str(POLICY_KEY);
    NSUserDefaults::standardUserDefaults()
        .stringForKey(&key)
        .as_deref()
        .and_then(|value| AppLockPolicy::parse(&value.to_string()))
        .unwrap_or_default()
}

pub fn store_app_lock_policy(policy: AppLockPolicy) {
    let defaults = NSUserDefaults::standardUserDefaults();
    let key = NSString::from_str(POLICY_KEY);
    let value = NSString::from_str(policy.as_str());
    // SAFETY: NSString is a supported NSUserDefaults property-list value.
    unsafe { defaults.setObject_forKey(Some(&value), &key) };
}

#[must_use]
pub fn app_lock_requires_authentication() -> bool {
    let defaults = NSUserDefaults::standardUserDefaults();
    if !defaults.boolForKey(&NSString::from_str(SETUP_COMPLETE_KEY)) {
        return false;
    }
    match app_lock_policy() {
        AppLockPolicy::EveryLaunch => true,
        AppLockPolicy::OncePerBoot => {
            let authenticated_boot =
                defaults.doubleForKey(&NSString::from_str(AUTHENTICATED_BOOT_KEY));
            (authenticated_boot - current_boot_marker()).abs() > BOOT_MARKER_TOLERANCE_SECS
        }
        AppLockPolicy::Off => false,
    }
}

pub fn record_app_lock_authentication() {
    NSUserDefaults::standardUserDefaults()
        .setDouble_forKey(current_boot_marker(), &NSString::from_str(AUTHENTICATED_BOOT_KEY));
}

pub fn record_app_lock_setup_complete() {
    NSUserDefaults::standardUserDefaults()
        .setBool_forKey(true, &NSString::from_str(SETUP_COMPLETE_KEY));
}

fn current_boot_marker() -> f64 {
    NSDate::date().timeIntervalSince1970() - NSProcessInfo::processInfo().systemUptime()
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
