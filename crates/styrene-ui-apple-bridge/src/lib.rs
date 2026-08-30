//! Audited Objective-C boundary for Apple platform APIs.
//!
//! No Objective-C object or pointer crosses this crate's safe public API.

mod ble;
#[cfg(target_os = "ios")]
mod ble_ios;

pub use ble::*;
#[cfg(target_os = "ios")]
pub use ble_ios::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeAuthorization {
    NotDetermined,
    Granted,
    Denied,
    Restricted,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeBridgeFailure {
    MediaTypeUnavailable,
}

#[cfg(target_os = "ios")]
mod ios {
    use std::ptr::NonNull;
    use std::sync::{Mutex, Once};

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{Bool, ProtocolObject};
    use objc2::{AnyThread, DefinedClass, define_class, msg_send};
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeVideo};
    use objc2_core_bluetooth::{
        CBCentralManager, CBCentralManagerDelegate, CBManager, CBManagerAuthorization,
    };
    use objc2_foundation::{NSNotification, NSNotificationCenter, NSObject, NSObjectProtocol};
    use objc2_ui_kit::UIContentSizeCategoryDidChangeNotification;
    use objc2_user_notifications::{
        UNAuthorizationStatus, UNNotificationSettings, UNUserNotificationCenter,
    };

    use super::{NativeAuthorization, NativeBridgeFailure};

    type AuthorizationCallback = Box<dyn Fn(NativeAuthorization) + Send + Sync + 'static>;
    static CONTENT_SIZE_OBSERVER: Once = Once::new();

    struct BluetoothDelegateIvars {
        callback: Mutex<Option<AuthorizationCallback>>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and the ivars are
        // initialized before `init` returns.
        #[unsafe(super = NSObject)]
        #[ivars = BluetoothDelegateIvars]
        struct BluetoothDelegate;

        // SAFETY: NSObjectProtocol has no additional implementation requirements.
        unsafe impl NSObjectProtocol for BluetoothDelegate {}

        // SAFETY: The selector and argument type match CBCentralManagerDelegate.
        unsafe impl CBCentralManagerDelegate for BluetoothDelegate {
            #[unsafe(method(centralManagerDidUpdateState:))]
            unsafe fn central_manager_did_update_state(&self, _: &CBCentralManager) {
                let authorization = bluetooth_authorization();
                if authorization != NativeAuthorization::NotDetermined
                    && let Ok(mut callback) = self.ivars().callback.lock()
                    && let Some(callback) = callback.take()
                {
                    callback(authorization);
                }
            }
        }
    );

    impl BluetoothDelegate {
        fn new(callback: AuthorizationCallback) -> Retained<Self> {
            let this = Self::alloc()
                .set_ivars(BluetoothDelegateIvars { callback: Mutex::new(Some(callback)) });
            // SAFETY: This sends NSObject's parameterless init to a fully
            // allocated object whose Rust ivars have been initialized.
            unsafe { msg_send![super(this), init] }
        }
    }

    pub struct BluetoothAuthorizationRequest {
        _delegate: Retained<BluetoothDelegate>,
        _manager: Retained<CBCentralManager>,
    }

    pub fn camera_authorization() -> NativeAuthorization {
        let Some(media_type) = video_media_type() else {
            return NativeAuthorization::Unavailable;
        };
        // SAFETY: AVMediaTypeVideo is the documented media type accepted by
        // this class method. The call returns a value and retains no reference.
        map_camera(unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type) })
    }

    pub fn request_camera<F>(callback: F) -> Result<(), NativeBridgeFailure>
    where
        F: Fn(NativeAuthorization) + Send + Sync + 'static,
    {
        let media_type = video_media_type().ok_or(NativeBridgeFailure::MediaTypeUnavailable)?;
        let completion = RcBlock::new(move |granted: Bool| {
            callback(if granted.as_bool() {
                NativeAuthorization::Granted
            } else {
                camera_authorization()
            });
        });
        // SAFETY: AVMediaTypeVideo is valid for this API. The escaping block is
        // heap-backed, captures only Send + Sync state, and is copied by Apple.
        unsafe {
            AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type, &completion);
        }
        Ok(())
    }

    pub fn bluetooth_authorization() -> NativeAuthorization {
        // SAFETY: This class property returns a value and retains no reference.
        map_bluetooth(unsafe { CBManager::authorization_class() })
    }

    pub fn request_bluetooth<F>(
        callback: F,
    ) -> Result<BluetoothAuthorizationRequest, NativeBridgeFailure>
    where
        F: Fn(NativeAuthorization) + Send + Sync + 'static,
    {
        let delegate = BluetoothDelegate::new(Box::new(callback));
        // SAFETY: The returned request token retains both manager and delegate.
        // A nil queue selects Apple's main dispatch queue.
        let manager = unsafe {
            CBCentralManager::initWithDelegate_queue(
                CBCentralManager::alloc(),
                Some(ProtocolObject::from_ref(&*delegate)),
                None,
            )
        };
        Ok(BluetoothAuthorizationRequest { _delegate: delegate, _manager: manager })
    }

    pub fn query_notification_authorization<F>(callback: F)
    where
        F: Fn(NativeAuthorization) + Send + Sync + 'static,
    {
        let completion = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
            // SAFETY: Apple's completion contract supplies a non-null settings
            // object valid for this callback. It is not retained or exposed.
            let settings = unsafe { settings.as_ref() };
            callback(map_notification(settings.authorizationStatus()));
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .getNotificationSettingsWithCompletionHandler(&completion);
    }

    pub fn install_content_size_observer<F>(callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        CONTENT_SIZE_OBSERVER.call_once(move || {
            let block = RcBlock::new(move |_: NonNull<NSNotification>| callback());
            // SAFETY: The notification name is Apple's immutable Dynamic Type
            // constant, the object filter is nil, and the block is Send + Sync.
            // The returned observer is deliberately process-scoped and bounded
            // to this one-time registration.
            let observer = unsafe {
                NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                    Some(UIContentSizeCategoryDidChangeNotification),
                    None,
                    None,
                    &block,
                )
            };
            std::mem::forget(observer);
        });
    }

    fn video_media_type() -> Option<&'static objc2_av_foundation::AVMediaType> {
        // SAFETY: This imports Apple's immutable AVMediaTypeVideo framework
        // constant. The Option models weak-link availability.
        unsafe { AVMediaTypeVideo }
    }

    fn map_camera(status: AVAuthorizationStatus) -> NativeAuthorization {
        match status {
            AVAuthorizationStatus::NotDetermined => NativeAuthorization::NotDetermined,
            AVAuthorizationStatus::Authorized => NativeAuthorization::Granted,
            AVAuthorizationStatus::Denied => NativeAuthorization::Denied,
            AVAuthorizationStatus::Restricted => NativeAuthorization::Restricted,
            _ => NativeAuthorization::Unavailable,
        }
    }

    fn map_bluetooth(status: CBManagerAuthorization) -> NativeAuthorization {
        match status {
            CBManagerAuthorization::NotDetermined => NativeAuthorization::NotDetermined,
            CBManagerAuthorization::AllowedAlways => NativeAuthorization::Granted,
            CBManagerAuthorization::Denied => NativeAuthorization::Denied,
            CBManagerAuthorization::Restricted => NativeAuthorization::Restricted,
            _ => NativeAuthorization::Unavailable,
        }
    }

    fn map_notification(status: UNAuthorizationStatus) -> NativeAuthorization {
        match status {
            UNAuthorizationStatus::NotDetermined => NativeAuthorization::NotDetermined,
            UNAuthorizationStatus::Authorized
            | UNAuthorizationStatus::Provisional
            | UNAuthorizationStatus::Ephemeral => NativeAuthorization::Granted,
            UNAuthorizationStatus::Denied => NativeAuthorization::Denied,
            _ => NativeAuthorization::Unavailable,
        }
    }
}

#[cfg(target_os = "ios")]
pub use ios::BluetoothAuthorizationRequest;

#[cfg(target_os = "ios")]
pub use ios::{
    bluetooth_authorization, camera_authorization, install_content_size_observer,
    query_notification_authorization, request_bluetooth, request_camera,
};

#[cfg(not(target_os = "ios"))]
pub struct BluetoothAuthorizationRequest;

#[cfg(not(target_os = "ios"))]
#[must_use]
pub const fn camera_authorization() -> NativeAuthorization {
    NativeAuthorization::Unavailable
}

#[cfg(not(target_os = "ios"))]
#[must_use]
pub const fn bluetooth_authorization() -> NativeAuthorization {
    NativeAuthorization::Unavailable
}
