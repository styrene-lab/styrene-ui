//! Audited Objective-C boundary for Apple platform APIs.
//!
//! No Objective-C object or pointer crosses this crate's safe public API.

mod ble;
#[cfg(target_os = "ios")]
mod ble_ios;
#[cfg(target_os = "ios")]
mod device_authentication;
mod firmware;
#[cfg(target_os = "ios")]
mod firmware_ios;
mod legacy_dfu;
mod legacy_dfu_transport;

pub use ble::*;
#[cfg(target_os = "ios")]
pub use ble_ios::*;
#[cfg(target_os = "ios")]
pub use device_authentication::*;
pub use firmware::*;
#[cfg(target_os = "ios")]
pub use firmware_ios::*;
pub use legacy_dfu::*;
pub use legacy_dfu_transport::*;

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
    MainThreadUnavailable,
    Oversized,
    WriteFailed,
    PresentationUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeDocumentPickerFailure {
    Cancelled,
    Oversized,
    ReadFailed,
    PresentationUnavailable,
}

#[cfg(target_os = "ios")]
mod ios {
    use std::ptr::NonNull;
    use std::sync::{Mutex, Once};

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Bool, ProtocolObject};
    use objc2::{
        AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    };
    use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaTypeVideo};
    use objc2_core_bluetooth::{
        CBCentralManager, CBCentralManagerDelegate, CBManager, CBManagerAuthorization,
    };
    use objc2_foundation::{
        NSArray, NSData, NSDataReadingOptions, NSError, NSFileManager, NSNotification,
        NSNotificationCenter, NSObject, NSObjectProtocol, NSString, NSURL,
    };
    #[allow(deprecated)]
    use objc2_ui_kit::UIDocumentPickerMode;
    use objc2_ui_kit::{
        UIActivityType, UIActivityViewController, UIApplication,
        UIApplicationOpenSettingsURLString, UIContentSizeCategoryDidChangeNotification,
        UIDocumentPickerDelegate, UIDocumentPickerViewController, UIPasteboard,
    };
    use objc2_user_notifications::{
        UNAuthorizationStatus, UNNotificationSettings, UNUserNotificationCenter,
    };

    use super::{NativeAuthorization, NativeBridgeFailure, NativeDocumentPickerFailure};

    type AuthorizationCallback = Box<dyn Fn(NativeAuthorization) + Send + Sync + 'static>;
    type DocumentPickerCallback =
        Box<dyn Fn(Result<Vec<u8>, NativeDocumentPickerFailure>) + Send + Sync + 'static>;
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

    struct DocumentPickerDelegateIvars {
        callback: Mutex<Option<DocumentPickerCallback>>,
        max_bytes: usize,
    }

    define_class!(
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        #[ivars = DocumentPickerDelegateIvars]
        struct DocumentPickerDelegate;

        unsafe impl NSObjectProtocol for DocumentPickerDelegate {}

        unsafe impl UIDocumentPickerDelegate for DocumentPickerDelegate {
            #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
            fn document_picker_did_pick_documents(
                &self,
                _: &UIDocumentPickerViewController,
                urls: &NSArray<NSURL>,
            ) {
                let result = urls
                    .firstObject()
                    .map_or(Err(NativeDocumentPickerFailure::ReadFailed), |url| {
                        read_identity_backup(&url, self.ivars().max_bytes)
                    });
                self.complete(result);
            }

            #[unsafe(method(documentPickerWasCancelled:))]
            fn document_picker_was_cancelled(&self, _: &UIDocumentPickerViewController) {
                self.complete(Err(NativeDocumentPickerFailure::Cancelled));
            }
        }
    );

    impl DocumentPickerDelegate {
        fn new(
            marker: MainThreadMarker,
            max_bytes: usize,
            callback: DocumentPickerCallback,
        ) -> Retained<Self> {
            let this = marker.alloc().set_ivars(DocumentPickerDelegateIvars {
                callback: Mutex::new(Some(callback)),
                max_bytes,
            });
            // SAFETY: NSObject's parameterless initializer accepts the fully
            // initialized Rust ivars above.
            unsafe { msg_send![super(this), init] }
        }

        fn complete(&self, result: Result<Vec<u8>, NativeDocumentPickerFailure>) {
            if let Ok(mut callback) = self.ivars().callback.lock()
                && let Some(callback) = callback.take()
            {
                callback(result);
            }
        }
    }

    pub struct BluetoothAuthorizationRequest {
        _delegate: Retained<BluetoothDelegate>,
        _manager: Retained<CBCentralManager>,
    }

    pub struct IdentityBackupPicker {
        _delegate: Retained<DocumentPickerDelegate>,
        _controller: Retained<UIDocumentPickerViewController>,
    }

    pub fn present_identity_backup_picker<F>(
        max_bytes: usize,
        callback: F,
    ) -> Result<IdentityBackupPicker, NativeDocumentPickerFailure>
    where
        F: Fn(Result<Vec<u8>, NativeDocumentPickerFailure>) + Send + Sync + 'static,
    {
        let marker =
            MainThreadMarker::new().ok_or(NativeDocumentPickerFailure::PresentationUnavailable)?;
        #[allow(deprecated)]
        let document_types = NSArray::from_slice(&[&*NSString::from_str("public.data")]);
        #[allow(deprecated)]
        let controller = UIDocumentPickerViewController::initWithDocumentTypes_inMode(
            marker.alloc(),
            &document_types,
            UIDocumentPickerMode::Import,
        );
        controller.setAllowsMultipleSelection(false);
        let delegate = DocumentPickerDelegate::new(marker, max_bytes, Box::new(callback));
        controller.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

        #[allow(deprecated)]
        let Some(window) = UIApplication::sharedApplication(marker).keyWindow() else {
            return Err(NativeDocumentPickerFailure::PresentationUnavailable);
        };
        let mut presenter = window
            .rootViewController()
            .ok_or(NativeDocumentPickerFailure::PresentationUnavailable)?;
        while let Some(presented) = presenter.presentedViewController() {
            presenter = presented;
        }
        presenter.presentViewController_animated_completion(&controller, true, None);
        Ok(IdentityBackupPicker { _delegate: delegate, _controller: controller })
    }

    fn read_identity_backup(
        url: &NSURL,
        max_bytes: usize,
    ) -> Result<Vec<u8>, NativeDocumentPickerFailure> {
        if url.pathExtension().as_deref().map(NSString::to_string).as_deref() != Some("stid") {
            return Err(NativeDocumentPickerFailure::ReadFailed);
        }
        // SAFETY: Access is balanced before returning and the URL is retained
        // by the delegate callback for the duration of this synchronous read.
        let scoped = unsafe { url.startAccessingSecurityScopedResource() };
        let result = (|| {
            let path = url.path().ok_or(NativeDocumentPickerFailure::ReadFailed)?;
            let attributes = NSFileManager::defaultManager()
                .attributesOfItemAtPath_error(&path)
                .map_err(|_| NativeDocumentPickerFailure::ReadFailed)?;
            if usize::try_from(attributes.fileSize()).map_or(true, |size| size > max_bytes) {
                return Err(NativeDocumentPickerFailure::Oversized);
            }
            let data = NSData::dataWithContentsOfURL_options_error(
                url,
                NSDataReadingOptions::MappedAlways,
            )
            .map_err(|_| NativeDocumentPickerFailure::ReadFailed)?;
            if data.length() > max_bytes {
                return Err(NativeDocumentPickerFailure::Oversized);
            }
            // SAFETY: NSData owns an immutable buffer for this call. The bytes
            // are copied into Rust-owned bounded storage before NSData drops.
            Ok(unsafe { data.as_bytes_unchecked() }.to_vec())
        })();
        if scoped {
            // SAFETY: This balances the successful access call above.
            unsafe { url.stopAccessingSecurityScopedResource() };
        }
        result
    }

    /// Read plain clipboard text on the main thread without exposing Objective-C objects.
    pub fn clipboard_text(max_bytes: usize) -> Result<Option<Vec<u8>>, NativeBridgeFailure> {
        let _marker = MainThreadMarker::new().ok_or(NativeBridgeFailure::MainThreadUnavailable)?;
        let pasteboard = UIPasteboard::generalPasteboard();
        // SAFETY: MainThreadMarker above proves this call runs on the main
        // thread. The returned NSString is retained and does not escape.
        let Some(text) = (unsafe { pasteboard.string() }) else {
            return Ok(None);
        };
        if text.len() > max_bytes {
            return Err(NativeBridgeFailure::Oversized);
        }
        Ok(Some(text.to_string().into_bytes()))
    }

    /// Write plain public text on the main thread without exposing Objective-C objects.
    pub fn set_clipboard_text(value: &str) -> Result<(), NativeBridgeFailure> {
        let _marker = MainThreadMarker::new().ok_or(NativeBridgeFailure::MainThreadUnavailable)?;
        let value = NSString::from_str(value);
        // SAFETY: MainThreadMarker above proves UIKit main-thread access, and
        // UIPasteboard copies the NSString rather than retaining a Rust borrow.
        unsafe { UIPasteboard::generalPasteboard().setString(Some(&value)) };
        Ok(())
    }

    pub fn open_application_settings() -> Result<bool, NativeBridgeFailure> {
        let marker = MainThreadMarker::new().ok_or(NativeBridgeFailure::MainThreadUnavailable)?;
        // SAFETY: UIKit initializes this process-wide NSString constant before
        // application code runs; the reference does not escape this call.
        let settings_url = unsafe { UIApplicationOpenSettingsURLString };
        let url =
            NSURL::URLWithString(settings_url).ok_or(NativeBridgeFailure::MediaTypeUnavailable)?;
        #[allow(deprecated)]
        Ok(UIApplication::sharedApplication(marker).openURL(&url))
    }

    /// Present an opaque encrypted identity artifact through the iOS share sheet.
    pub fn present_identity_backup<F>(bytes: &[u8], presented: F) -> Result<(), NativeBridgeFailure>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let marker = MainThreadMarker::new().ok_or(NativeBridgeFailure::MainThreadUnavailable)?;
        let filename = NSString::from_str("styrene-identity-backup.stid");
        let file_manager = NSFileManager::defaultManager();
        let url = file_manager
            .temporaryDirectory()
            .URLByAppendingPathComponent(&filename)
            .ok_or(NativeBridgeFailure::WriteFailed)?;
        let _ = file_manager.removeItemAtURL_error(&url);
        // SAFETY: `bytes` remains valid for this call and NSData copies the
        // complete bounded slice before the function continues.
        let data = unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };
        if !data.writeToURL_atomically(&url, true) {
            return Err(NativeBridgeFailure::WriteFailed);
        }

        #[allow(deprecated)]
        let Some(window) = UIApplication::sharedApplication(marker).keyWindow() else {
            let _ = file_manager.removeItemAtURL_error(&url);
            return Err(NativeBridgeFailure::PresentationUnavailable);
        };
        let mut presenter =
            window.rootViewController().ok_or(NativeBridgeFailure::PresentationUnavailable)?;
        while let Some(presented) = presenter.presentedViewController() {
            presenter = presented;
        }
        // SAFETY: NSURL is an Objective-C object and UIActivityViewController
        // accepts heterogeneous Objective-C activity items.
        let activity_item = unsafe { Retained::<NSURL>::cast_unchecked::<AnyObject>(url.clone()) };
        let items = NSArray::from_slice(&[&*activity_item]);
        // SAFETY: The activity item is a retained file URL and no custom
        // activities are supplied. UIKit retains the controller while shown.
        let controller = unsafe {
            UIActivityViewController::initWithActivityItems_applicationActivities(
                UIActivityViewController::alloc(marker),
                &items,
                None,
            )
        };
        let cleanup_url = url.clone();
        let cleanup = RcBlock::new(
            move |_: *mut UIActivityType, _: Bool, _: *mut NSArray, _: *mut NSError| {
                let _ = NSFileManager::defaultManager().removeItemAtURL_error(&cleanup_url);
            },
        );
        // SAFETY: The block signature exactly matches UIKit's completion type.
        // UIKit copies the block and invokes it after the share sheet closes.
        unsafe {
            controller.setCompletionWithItemsHandler(
                (&*cleanup as *const block2::DynBlock<_>).cast_mut(),
            );
        }
        if let Some(popover) = controller.popoverPresentationController()
            && let Some(view) = presenter.view()
        {
            popover.setSourceView(Some(&view));
            popover.setSourceRect(view.bounds());
        }
        let presentation = RcBlock::new(presented);
        presenter.presentViewController_animated_completion(&controller, true, Some(&presentation));
        Ok(())
    }

    pub fn remove_identity_backup_temp_file() -> Result<(), NativeBridgeFailure> {
        let filename = NSString::from_str("styrene-identity-backup.stid");
        let file_manager = NSFileManager::defaultManager();
        let url = file_manager
            .temporaryDirectory()
            .URLByAppendingPathComponent(&filename)
            .ok_or(NativeBridgeFailure::WriteFailed)?;
        let _ = file_manager.removeItemAtURL_error(&url);
        Ok(())
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
pub use ios::{BluetoothAuthorizationRequest, IdentityBackupPicker};

#[cfg(target_os = "ios")]
pub use ios::{
    bluetooth_authorization, camera_authorization, clipboard_text, install_content_size_observer,
    open_application_settings, present_identity_backup, present_identity_backup_picker,
    query_notification_authorization, remove_identity_backup_temp_file, request_bluetooth,
    request_camera, set_clipboard_text,
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
