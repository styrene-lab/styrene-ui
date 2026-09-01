use std::sync::mpsc;

use block2::RcBlock;
use objc2::{AnyThread, runtime::Bool};
use objc2_foundation::{NSError, NSString};
use objc2_local_authentication::{LAContext, LAError, LAPolicy};
use styrene_ui_platform::DeviceAuthenticationOutcome;

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
