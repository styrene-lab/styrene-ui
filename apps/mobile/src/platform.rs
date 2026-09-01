use dioxus::prelude::*;
use serde::Deserialize;
#[cfg(target_os = "ios")]
use styrene_ui_platform::DocumentPickerFailure;
#[cfg(not(target_os = "android"))]
use styrene_ui_platform::DocumentShareFailure;
use styrene_ui_platform::{
    AccessibilityPreferences, AndroidUsbAttachment, Appearance, ApplicationLifecycle,
    AuthorizationState, ClipboardTextReader, ClipboardTextWriter, Contrast,
    DocumentPickerCompletion, DocumentRequestGeneration, DocumentShareCompletion, KeyboardGeometry,
    MotionPreference, OpaqueDocument, OpaqueDocumentPicker, OpaqueDocumentSharer, PermissionKind,
    PermissionStatus, PlatformApplyResult, PlatformChange, PlatformEvent, PlatformEventStream,
    PlatformFailure, PlatformFuture, PlatformGeometry, PlatformInsets, PlatformService,
    PlatformSnapshot, PlatformState, TextAcquisitionCompletion, TextAcquisitionGeneration,
    TextScale, WindowClass, WindowMetrics,
};

#[cfg(any(test, target_os = "android"))]
mod android_policy {
    pub const CAMERA: &str = "android.permission.CAMERA";
    pub const LOCATION: &str = "android.permission.ACCESS_FINE_LOCATION";
    pub const BLUETOOTH_SCAN: &str = "android.permission.BLUETOOTH_SCAN";
    pub const BLUETOOTH_CONNECT: &str = "android.permission.BLUETOOTH_CONNECT";
    pub const POST_NOTIFICATIONS: &str = "android.permission.POST_NOTIFICATIONS";

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn font_scale_percent(scale: f32) -> Option<u16> {
        if !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        Some((scale * 100.0).round().clamp(1.0, f32::from(u16::MAX)) as u16)
    }

    pub fn permission_names(
        kind: styrene_ui_platform::PermissionKind,
        sdk: i32,
    ) -> &'static [&'static str] {
        use styrene_ui_platform::PermissionKind;
        match (kind, sdk) {
            (PermissionKind::Bluetooth, 31..) => &[BLUETOOTH_SCAN, BLUETOOTH_CONNECT],
            (PermissionKind::Bluetooth, _) => &[LOCATION],
            (PermissionKind::Camera, _) => &[CAMERA],
            (PermissionKind::Usb, _) => &[],
        }
    }

    pub fn notification_permission_names(sdk: i32) -> &'static [&'static str] {
        if sdk >= 33 { &[POST_NOTIFICATIONS] } else { &[] }
    }

    pub const fn merge_authorization(
        current: styrene_ui_platform::AuthorizationState,
        next: styrene_ui_platform::AuthorizationState,
    ) -> styrene_ui_platform::AuthorizationState {
        use styrene_ui_platform::AuthorizationState::{
            Denied, Granted, NotDetermined, Restricted, Unavailable,
        };
        match (current, next) {
            (Restricted, _) | (_, Restricted) => Restricted,
            (Denied, _) | (_, Denied) => Denied,
            (NotDetermined, _) | (_, NotDetermined) => NotDetermined,
            (Unavailable, _) | (_, Unavailable) => Unavailable,
            (Granted, Granted) => Granted,
        }
    }

    pub const fn document_share_result(
        status: i32,
    ) -> Result<styrene_ui_platform::DocumentShareOutcome, styrene_ui_platform::DocumentShareFailure>
    {
        match status {
            0 => Ok(styrene_ui_platform::DocumentShareOutcome::Presented),
            1 => Err(styrene_ui_platform::DocumentShareFailure::Unavailable),
            _ => Err(styrene_ui_platform::DocumentShareFailure::PresentationFailed),
        }
    }
}

#[cfg(any(test, target_os = "ios"))]
fn ios_text_scale_category(raw: &str) -> TextScale {
    use styrene_ui_platform::TextScaleCategory;
    let category = match raw {
        "UICTContentSizeCategoryXS" => TextScaleCategory::ExtraSmall,
        "UICTContentSizeCategoryS" => TextScaleCategory::Small,
        "UICTContentSizeCategoryM" => TextScaleCategory::Medium,
        "UICTContentSizeCategoryL" => TextScaleCategory::Large,
        "UICTContentSizeCategoryXL" => TextScaleCategory::ExtraLarge,
        "UICTContentSizeCategoryXXL" => TextScaleCategory::ExtraExtraLarge,
        "UICTContentSizeCategoryXXXL" => TextScaleCategory::ExtraExtraExtraLarge,
        "UICTContentSizeCategoryAccessibilityM" => TextScaleCategory::AccessibilityMedium,
        "UICTContentSizeCategoryAccessibilityL" => TextScaleCategory::AccessibilityLarge,
        "UICTContentSizeCategoryAccessibilityXL" => TextScaleCategory::AccessibilityExtraLarge,
        "UICTContentSizeCategoryAccessibilityXXL" => {
            TextScaleCategory::AccessibilityExtraExtraLarge
        }
        "UICTContentSizeCategoryAccessibilityXXXL" => {
            TextScaleCategory::AccessibilityExtraExtraExtraLarge
        }
        "UICTContentSizeCategoryUnspecified" | "" => return TextScale::Unavailable,
        _ => TextScaleCategory::Unknown,
    };
    TextScale::Category(category)
}

#[cfg(target_os = "android")]
mod native_platform {
    use super::android_policy::{
        font_scale_percent, merge_authorization, notification_permission_names, permission_names,
    };
    use async_channel::Sender;
    use jni::{
        JNIEnv,
        objects::{JObject, JString, JValue},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    };
    use styrene_ui_platform::{
        AndroidUsbAttachment, AuthorizationState, CandidatePayload, DocumentPickerFailure,
        MAX_CANDIDATE_PAYLOAD_BYTES, MAX_OPAQUE_DOCUMENT_BYTES, PermissionKind, PermissionStatus,
        PlatformFailure, PlatformSnapshot, TextAcquisitionCompletion, TextAcquisitionFailure,
        TextAcquisitionGeneration, TextScale,
    };

    static PERMISSION_REQUEST_ACTIVE: AtomicBool = AtomicBool::new(false);
    static DOCUMENT_REQUEST_ACTIVE: AtomicBool = AtomicBool::new(false);
    static USB_REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

    enum ClipboardRead {
        Payload(Vec<u8>),
        Oversized,
    }

    enum DocumentRead {
        Bytes(Vec<u8>),
        Oversized,
        InvalidType,
    }

    struct PermissionRequestGuard;

    impl PermissionRequestGuard {
        fn acquire() -> Result<Self, PlatformFailure> {
            PERMISSION_REQUEST_ACTIVE
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| Self)
                .map_err(|_| failure("android_permission_request_busy", true))
        }
    }

    impl Drop for PermissionRequestGuard {
        fn drop(&mut self) {
            PERMISSION_REQUEST_ACTIVE.store(false, Ordering::Release);
        }
    }

    struct UsbPermissionCleanup {
        cancelled: Arc<AtomicBool>,
        armed: bool,
    }

    impl UsbPermissionCleanup {
        fn complete(&mut self) {
            wry::clear_usb_permission_result_handler();
            self.armed = false;
        }

        fn cancel(&mut self) {
            if !self.armed {
                return;
            }
            self.cancelled.store(true, Ordering::Release);
            wry::clear_usb_permission_result_handler();
            let _ = wry::try_dispatch(|env, activity, _| {
                let _ = cancel_usb_permission_request(env, activity);
                if env.exception_check().unwrap_or(false) {
                    let _ = env.exception_clear();
                }
            });
            self.armed = false;
        }
    }

    impl Drop for UsbPermissionCleanup {
        fn drop(&mut self) {
            self.cancel();
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn prepare_subscription() -> Option<async_channel::Receiver<()>> {
        let (sender, receiver) = async_channel::bounded(1);
        wry::set_configuration_changed_handler(move || {
            let _ = sender.try_send(());
        });
        Some(receiver)
    }

    #[derive(Clone, Debug)]
    struct NativeFacts {
        font_scale: Option<u16>,
        camera: AuthorizationState,
        bluetooth: AuthorizationState,
        usb: AuthorizationState,
        notifications: AuthorizationState,
    }

    pub async fn enrich_snapshot(snapshot: &mut PlatformSnapshot) {
        snapshot.permissions =
            [PermissionKind::Bluetooth, PermissionKind::Camera, PermissionKind::Usb]
                .into_iter()
                .map(|kind| PermissionStatus { kind, state: AuthorizationState::Unavailable })
                .collect();
        let Ok(facts) = dispatch_query(read_native_facts).await else {
            return;
        };
        snapshot.accessibility.text_scale =
            facts.font_scale.map_or(TextScale::Unavailable, TextScale::Percent);
        snapshot.permissions = vec![
            PermissionStatus { kind: PermissionKind::Bluetooth, state: facts.bluetooth },
            PermissionStatus { kind: PermissionKind::Camera, state: facts.camera },
            PermissionStatus { kind: PermissionKind::Usb, state: facts.usb },
        ];
        snapshot.notification_authorization = facts.notifications;
    }

    pub async fn request_permission(
        kind: PermissionKind,
    ) -> Result<PermissionStatus, PlatformFailure> {
        if kind == PermissionKind::Usb {
            return Ok(PermissionStatus { kind, state: AuthorizationState::Unavailable });
        }
        let sdk = dispatch_query(read_sdk).await?;
        let names = permission_names(kind, sdk);
        request_and_observe(names).await.map(|state| PermissionStatus { kind, state })
    }

    pub async fn request_notifications() -> Result<AuthorizationState, PlatformFailure> {
        let sdk = dispatch_query(read_sdk).await?;
        let names = notification_permission_names(sdk);
        if names.is_empty() {
            return dispatch_query(move |env, activity| notifications_state(env, activity, sdk))
                .await;
        }
        request_and_observe(names).await?;
        dispatch_query(move |env, activity| notifications_state(env, activity, sdk)).await
    }

    pub async fn open_application_settings() -> Result<(), PlatformFailure> {
        dispatch_query(|env, activity| {
            let action = env.new_string("android.settings.APPLICATION_DETAILS_SETTINGS")?;
            let action = JObject::from(action);
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&action)],
            )?;
            let package =
                env.call_method(activity, "getPackageName", "()Ljava/lang/String;", &[])?.l()?;
            let package = JString::from(package);
            let package = env.get_string(&package)?.to_string_lossy().into_owned();
            let uri = env.new_string(format!("package:{package}"))?;
            let uri = JObject::from(uri);
            let uri = env
                .call_static_method(
                    "android/net/Uri",
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&uri)],
                )?
                .l()?;
            env.call_method(
                &intent,
                "setData",
                "(Landroid/net/Uri;)Landroid/content/Intent;",
                &[JValue::Object(&uri)],
            )?;
            env.call_method(
                activity,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        })
        .await
    }

    #[allow(dead_code)] // Consumed by the compose integration in the adjacent workflow task.
    pub async fn read_clipboard_text(
        generation: TextAcquisitionGeneration,
    ) -> TextAcquisitionCompletion {
        let result = dispatch_query(read_clipboard_bytes)
            .await
            .map_err(|_| TextAcquisitionFailure::Unavailable)
            .and_then(|value| value.ok_or(TextAcquisitionFailure::Unavailable))
            .and_then(|value| match value {
                ClipboardRead::Payload(value) => {
                    CandidatePayload::from_service_bytes(value).map_err(Into::into)
                }
                ClipboardRead::Oversized => Err(TextAcquisitionFailure::Oversized),
            });
        TextAcquisitionCompletion { generation, result }
    }

    pub async fn write_clipboard_text(value: String) -> Result<(), PlatformFailure> {
        dispatch_query(move |env, activity| set_clipboard_text(env, activity, &value)).await
    }

    pub async fn pick_identity_backup() -> Result<Vec<u8>, DocumentPickerFailure> {
        DOCUMENT_REQUEST_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| DocumentPickerFailure::Unavailable)?;
        let (sender, receiver) = async_channel::bounded(1);
        wry::set_document_picker_result_handler(move |uri| {
            let _ = sender.try_send(uri);
        });
        let Ok(started) = dispatch_query(|env, activity| {
            env.call_method(activity, "requestIdentityBackupDocument", "()Z", &[])?.z()
        })
        .await
        else {
            wry::clear_document_picker_result_handler();
            DOCUMENT_REQUEST_ACTIVE.store(false, Ordering::Release);
            return Err(DocumentPickerFailure::Unavailable);
        };
        if !started {
            wry::clear_document_picker_result_handler();
            DOCUMENT_REQUEST_ACTIVE.store(false, Ordering::Release);
            return Err(DocumentPickerFailure::Unavailable);
        }
        let selected = tokio::time::timeout(std::time::Duration::from_mins(5), receiver.recv())
            .await
            .map_err(|_| DocumentPickerFailure::ReadFailed)
            .and_then(|result| result.map_err(|_| DocumentPickerFailure::ReadFailed));
        wry::clear_document_picker_result_handler();
        DOCUMENT_REQUEST_ACTIVE.store(false, Ordering::Release);
        let uri = selected?.ok_or(DocumentPickerFailure::Cancelled)?;
        match dispatch_query(move |env, activity| read_document_uri(env, activity, &uri))
            .await
            .map_err(|_| DocumentPickerFailure::ReadFailed)?
        {
            DocumentRead::Bytes(bytes) => Ok(bytes),
            DocumentRead::Oversized => Err(DocumentPickerFailure::Oversized),
            DocumentRead::InvalidType => Err(DocumentPickerFailure::ReadFailed),
        }
    }

    pub async fn share_identity_backup(
        document: Vec<u8>,
    ) -> Result<styrene_ui_platform::DocumentShareOutcome, styrene_ui_platform::DocumentShareFailure>
    {
        if document.len() > MAX_OPAQUE_DOCUMENT_BYTES {
            return Err(styrene_ui_platform::DocumentShareFailure::PresentationFailed);
        }
        let status = dispatch_query(move |env, activity| {
            let document = env.byte_array_from_slice(&document)?;
            env.call_method(
                activity,
                "presentIdentityBackup",
                "([B)I",
                &[JValue::Object(&document)],
            )?
            .i()
        })
        .await
        .map_err(|_| styrene_ui_platform::DocumentShareFailure::Unavailable)?;
        super::android_policy::document_share_result(status)
    }

    #[allow(clippy::too_many_lines)]
    fn read_document_uri(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        uri: &str,
    ) -> jni::errors::Result<DocumentRead> {
        let uri = env.new_string(uri)?;
        let uri = env
            .call_static_method(
                "android/net/Uri",
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&uri)],
            )?
            .l()?;
        let resolver = env
            .call_method(
                activity,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )?
            .l()?;
        let projection = env.new_object_array(2, "java/lang/String", JObject::null())?;
        let display_name = env.new_string("_display_name")?;
        let size = env.new_string("_size")?;
        env.set_object_array_element(&projection, 0, &display_name)?;
        env.set_object_array_element(&projection, 1, &size)?;
        let cursor = env
            .call_method(
                &resolver,
                "query",
                "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
                &[
                    JValue::Object(&uri),
                    JValue::Object(&projection),
                    JValue::Object(&JObject::null()),
                    JValue::Object(&JObject::null()),
                    JValue::Object(&JObject::null()),
                ],
            )?
            .l()?;
        if cursor.is_null() || !env.call_method(&cursor, "moveToFirst", "()Z", &[])?.z()? {
            return Ok(DocumentRead::InvalidType);
        }
        let name_index = env
            .call_method(
                &cursor,
                "getColumnIndex",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&display_name)],
            )?
            .i()?;
        let size_index = env
            .call_method(
                &cursor,
                "getColumnIndex",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&size)],
            )?
            .i()?;
        let name = if name_index >= 0 {
            env.call_method(
                &cursor,
                "getString",
                "(I)Ljava/lang/String;",
                &[JValue::Int(name_index)],
            )?
            .l()?
        } else {
            JObject::null()
        };
        let declared_size = if size_index >= 0 {
            env.call_method(&cursor, "getLong", "(I)J", &[JValue::Int(size_index)])?.j()?
        } else {
            -1
        };
        let _ = env.call_method(&cursor, "close", "()V", &[]);
        if name.is_null()
            || !env
                .get_string(&JString::from(name))?
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".stid")
        {
            return Ok(DocumentRead::InvalidType);
        }
        if declared_size > i64::try_from(MAX_OPAQUE_DOCUMENT_BYTES).unwrap_or(i64::MAX) {
            return Ok(DocumentRead::Oversized);
        }
        let stream = env
            .call_method(
                &resolver,
                "openInputStream",
                "(Landroid/net/Uri;)Ljava/io/InputStream;",
                &[JValue::Object(&uri)],
            )?
            .l()?;
        if stream.is_null() {
            return Ok(DocumentRead::InvalidType);
        }
        let buffer = env.new_byte_array(8192)?;
        let mut bytes = Vec::with_capacity(
            usize::try_from(declared_size.max(0)).unwrap_or(0).min(MAX_OPAQUE_DOCUMENT_BYTES),
        );
        loop {
            let read =
                env.call_method(&stream, "read", "([B)I", &[JValue::Object(&buffer)])?.i()?;
            if read < 0 {
                break;
            }
            let read = usize::try_from(read).map_err(|_| invalid_arguments())?;
            if bytes.len().saturating_add(read) > MAX_OPAQUE_DOCUMENT_BYTES {
                let _ = env.call_method(&stream, "close", "()V", &[]);
                return Ok(DocumentRead::Oversized);
            }
            let chunk = env.convert_byte_array(&buffer)?;
            bytes.extend_from_slice(&chunk[..read]);
        }
        env.call_method(&stream, "close", "()V", &[])?.v()?;
        Ok(DocumentRead::Bytes(bytes))
    }

    async fn request_and_observe(
        names: &'static [&'static str],
    ) -> Result<AuthorizationState, PlatformFailure> {
        let _request_guard = PermissionRequestGuard::acquire()?;
        let initially_granted =
            dispatch_query(move |env, activity| permissions_state(env, activity, names)).await?;
        if initially_granted == AuthorizationState::Granted {
            return Ok(initially_granted);
        }

        dispatch_query(move |env, activity| request_permissions(env, activity, names)).await?;
        let mut lost_focus = false;
        for attempt in 0..120 {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            let focused = dispatch_query(has_window_focus).await?;
            lost_focus |= !focused;
            if focused && (lost_focus || attempt >= 3) {
                return dispatch_query(move |env, activity| {
                    permissions_state(env, activity, names)
                })
                .await;
            }
        }
        Err(failure("android_permission_result_timeout", true))
    }

    async fn dispatch_query<T, F>(query: F) -> Result<T, PlatformFailure>
    where
        T: Send + 'static,
        F: FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<T> + Send + 'static,
    {
        let (sender, receiver) = async_channel::bounded(1);
        wry::try_dispatch(move |env, activity, _| {
            if activity.is_null() {
                send_result(&sender, Err(failure("android_activity_unavailable", true)));
                return;
            }
            let result = query(env, activity).map_err(|_| {
                if env.exception_check().unwrap_or(false) {
                    let _ = env.exception_clear();
                }
                failure("android_native_query_failed", true)
            });
            send_result(&sender, result);
        })
        .map_err(|_| failure("android_dispatch_unavailable", true))?;
        tokio::time::timeout(std::time::Duration::from_secs(5), receiver.recv())
            .await
            .map_err(|_| failure("android_dispatch_timeout", true))?
            .map_err(|_| failure("android_dispatch_closed", true))?
    }

    fn send_result<T>(
        sender: &Sender<Result<T, PlatformFailure>>,
        result: Result<T, PlatformFailure>,
    ) {
        let _ = sender.try_send(result);
    }

    fn read_native_facts(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
    ) -> jni::errors::Result<NativeFacts> {
        let sdk = read_sdk(env, activity)?;
        let resources = env
            .call_method(activity, "getResources", "()Landroid/content/res/Resources;", &[])?
            .l()?;
        let configuration = env
            .call_method(
                &resources,
                "getConfiguration",
                "()Landroid/content/res/Configuration;",
                &[],
            )?
            .l()?;
        let font_scale = font_scale_percent(env.get_field(&configuration, "fontScale", "F")?.f()?);
        Ok(NativeFacts {
            font_scale,
            camera: permissions_state(
                env,
                activity,
                permission_names(PermissionKind::Camera, sdk),
            )?,
            bluetooth: permissions_state(
                env,
                activity,
                permission_names(PermissionKind::Bluetooth, sdk),
            )?,
            usb: AuthorizationState::Unavailable,
            notifications: notifications_state(env, activity, sdk)?,
        })
    }

    fn read_clipboard_bytes(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
    ) -> jni::errors::Result<Option<ClipboardRead>> {
        let service_name = env.new_string("clipboard")?;
        let manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_name)],
            )?
            .l()?;
        if manager.is_null() || !env.call_method(&manager, "hasPrimaryClip", "()Z", &[])?.z()? {
            return Ok(None);
        }
        let clip = env
            .call_method(&manager, "getPrimaryClip", "()Landroid/content/ClipData;", &[])?
            .l()?;
        if clip.is_null() || env.call_method(&clip, "getItemCount", "()I", &[])?.i()? == 0 {
            return Ok(None);
        }
        let item = env
            .call_method(
                &clip,
                "getItemAt",
                "(I)Landroid/content/ClipData$Item;",
                &[JValue::Int(0)],
            )?
            .l()?;
        let text = env.call_method(&item, "getText", "()Ljava/lang/CharSequence;", &[])?.l()?;
        if text.is_null() {
            return Ok(None);
        }
        let text = env.call_method(&text, "toString", "()Ljava/lang/String;", &[])?.l()?;
        let utf16_len = env.call_method(&text, "length", "()I", &[])?.i()?;
        if usize::try_from(utf16_len).map_err(|_| invalid_arguments())?
            > MAX_CANDIDATE_PAYLOAD_BYTES
        {
            return Ok(Some(ClipboardRead::Oversized));
        }
        let text = env.get_string(&JString::from(text))?.to_string_lossy().into_owned();
        Ok(Some(ClipboardRead::Payload(text.into_bytes())))
    }

    fn set_clipboard_text(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        value: &str,
    ) -> jni::errors::Result<()> {
        let service_name = env.new_string("clipboard")?;
        let manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_name)],
            )?
            .l()?;
        let label = env.new_string("Public LXMF destination")?;
        let value = env.new_string(value)?;
        let clip = env
            .call_static_method(
                "android/content/ClipData",
                "newPlainText",
                "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
                &[JValue::Object(&label), JValue::Object(&value)],
            )?
            .l()?;
        env.call_method(
            &manager,
            "setPrimaryClip",
            "(Landroid/content/ClipData;)V",
            &[JValue::Object(&clip)],
        )?;
        Ok(())
    }

    fn read_sdk(env: &mut JNIEnv<'_>, _: &JObject<'_>) -> jni::errors::Result<i32> {
        env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?.i()
    }

    fn permissions_state(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        names: &[&str],
    ) -> jni::errors::Result<AuthorizationState> {
        let mut aggregate = AuthorizationState::Granted;
        for name in names {
            let requested = permission_was_requested(env, activity, name)?;
            let name = env.new_string(name)?;
            let granted = env
                .call_method(
                    activity,
                    "checkSelfPermission",
                    "(Ljava/lang/String;)I",
                    &[JValue::Object(&name)],
                )?
                .i()?
                == 0;
            if !granted {
                aggregate = merge_authorization(
                    aggregate,
                    if requested {
                        AuthorizationState::Denied
                    } else {
                        AuthorizationState::NotDetermined
                    },
                );
            }
        }
        Ok(aggregate)
    }

    fn permission_was_requested(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        permission: &str,
    ) -> jni::errors::Result<bool> {
        let preferences = permission_preferences(env, activity)?;
        let key = env.new_string(format!("permission_requested:{permission}"))?;
        env.call_method(
            &preferences,
            "getBoolean",
            "(Ljava/lang/String;Z)Z",
            &[JValue::Object(&key), JValue::Bool(0)],
        )?
        .z()
    }

    fn permission_preferences<'local>(
        env: &mut JNIEnv<'local>,
        activity: &JObject<'_>,
    ) -> jni::errors::Result<JObject<'local>> {
        let name = env.new_string("styrene.platform")?;
        env.call_method(
            activity,
            "getSharedPreferences",
            "(Ljava/lang/String;I)Landroid/content/SharedPreferences;",
            &[JValue::Object(&name), JValue::Int(0)],
        )?
        .l()
    }

    fn mark_permissions_requested(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        names: &[&str],
    ) -> jni::errors::Result<()> {
        let preferences = permission_preferences(env, activity)?;
        let editor = env
            .call_method(&preferences, "edit", "()Landroid/content/SharedPreferences$Editor;", &[])?
            .l()?;
        for name in names {
            let key = env.new_string(format!("permission_requested:{name}"))?;
            env.call_method(
                &editor,
                "putBoolean",
                "(Ljava/lang/String;Z)Landroid/content/SharedPreferences$Editor;",
                &[JValue::Object(&key), JValue::Bool(1)],
            )?;
        }
        env.call_method(&editor, "apply", "()V", &[])?.v()
    }

    fn notifications_state(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        sdk: i32,
    ) -> jni::errors::Result<AuthorizationState> {
        if sdk >= 33 {
            let runtime = permissions_state(env, activity, notification_permission_names(sdk))?;
            if runtime != AuthorizationState::Granted {
                return Ok(runtime);
            }
        }
        let service_name = env.new_string("notification")?;
        let manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service_name)],
            )?
            .l()?;
        let enabled = env.call_method(&manager, "areNotificationsEnabled", "()Z", &[])?.z()?;
        Ok(if enabled { AuthorizationState::Granted } else { AuthorizationState::Denied })
    }

    fn request_permissions(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        names: &[&str],
    ) -> jni::errors::Result<()> {
        mark_permissions_requested(env, activity, names)?;
        let class = env.find_class("java/lang/String")?;
        let initial = env.new_string("")?;
        let array = env.new_object_array(jni_int(names.len())?, class, &initial)?;
        for (index, name) in names.iter().enumerate() {
            let name = env.new_string(name)?;
            env.set_object_array_element(&array, jni_int(index)?, &name)?;
        }
        env.call_method(
            activity,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[JValue::Object(&array), JValue::Int(0x5354)],
        )?
        .v()
    }

    enum UsbRequestStart {
        Resolved(AuthorizationState),
        Requested(String),
    }

    pub async fn android_usb_attachments() -> Result<Vec<AndroidUsbAttachment>, PlatformFailure> {
        dispatch_query(enumerate_usb_attachments).await
    }

    pub async fn request_android_usb_authorization(
        attachment: AndroidUsbAttachment,
    ) -> Result<AuthorizationState, PlatformFailure> {
        let _request_guard = PermissionRequestGuard::acquire()?;
        let (sender, receiver) = async_channel::bounded(1);
        wry::set_usb_permission_result_handler(move |device_name, granted| {
            let _ = sender.try_send((device_name, granted));
        });
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut cleanup = UsbPermissionCleanup { cancelled: cancelled.clone(), armed: true };

        let requested_attachment = attachment.clone();
        let request_cancelled = cancelled;
        let start = dispatch_query(move |env, activity| {
            if request_cancelled.load(Ordering::Acquire) {
                return Err(jni::errors::Error::JniCall(jni::errors::JniError::InvalidArguments));
            }
            start_usb_permission_request(env, activity, &requested_attachment)
        })
        .await;
        let expected_name = match start {
            Ok(Some(UsbRequestStart::Resolved(state))) => {
                cleanup.complete();
                return Ok(state);
            }
            Ok(Some(UsbRequestStart::Requested(name))) => name,
            Ok(None) => {
                cleanup.cancel();
                return Err(failure("android_usb_attachment_stale", true));
            }
            Err(error) => {
                cleanup.cancel();
                return Err(error);
            }
        };

        let callback =
            tokio::time::timeout(std::time::Duration::from_mins(2), receiver.recv()).await;
        if callback.is_err() {
            cleanup.cancel();
        } else {
            cleanup.complete();
        }
        let (device_name, _granted) = callback
            .map_err(|_| failure("android_usb_permission_callback_timeout", true))?
            .map_err(|_| failure("android_usb_permission_callback_closed", true))?;
        if device_name != expected_name {
            return Err(failure("android_usb_permission_device_mismatch", false));
        }
        let state = dispatch_query(move |env, activity| {
            usb_state_for_attachment(env, activity, &attachment)
        })
        .await?;
        state.ok_or_else(|| failure("android_usb_attachment_stale", true))
    }

    fn start_usb_permission_request(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        attachment: &AndroidUsbAttachment,
    ) -> jni::errors::Result<Option<UsbRequestStart>> {
        let Some(device) = find_usb_device(env, activity, attachment)? else {
            return Ok(None);
        };
        if usb_has_permission(env, activity, &device)? {
            return Ok(Some(UsbRequestStart::Resolved(AuthorizationState::Granted)));
        }
        mark_usb_requested(env, activity, attachment)?;
        let action = env.new_string(format!(
            "io.styrene.mesh.USB_PERMISSION.{}.{}",
            attachment.device_id,
            USB_REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ))?;
        let started = env
            .call_method(
                activity,
                "requestUsbPermission",
                "(Landroid/hardware/usb/UsbDevice;Ljava/lang/String;)Z",
                &[JValue::Object(&device), JValue::Object(&action)],
            )?
            .z()?;
        if !started {
            return Err(invalid_arguments());
        }
        Ok(Some(UsbRequestStart::Requested(attachment.device_name.clone())))
    }

    fn cancel_usb_permission_request(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
    ) -> jni::errors::Result<()> {
        env.call_method(activity, "cancelUsbPermissionRequest", "()V", &[])?.v()
    }

    fn usb_state_for_attachment(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        attachment: &AndroidUsbAttachment,
    ) -> jni::errors::Result<Option<AuthorizationState>> {
        let Some(device) = find_usb_device(env, activity, attachment)? else {
            return Ok(None);
        };
        if usb_has_permission(env, activity, &device)? {
            return Ok(Some(AuthorizationState::Granted));
        }
        Ok(Some(if usb_was_requested(env, activity, attachment)? {
            AuthorizationState::Denied
        } else {
            AuthorizationState::NotDetermined
        }))
    }

    fn enumerate_usb_attachments(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
    ) -> jni::errors::Result<Vec<AndroidUsbAttachment>> {
        let iterator = usb_device_iterator(env, activity)?;
        let mut attachments = Vec::new();
        while env.call_method(&iterator, "hasNext", "()Z", &[])?.z()? {
            let device = env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[])?.l()?;
            attachments.push(usb_attachment(env, &device)?);
        }
        attachments.sort_by(|left, right| left.device_name.cmp(&right.device_name));
        Ok(attachments)
    }

    fn find_usb_device<'local>(
        env: &mut JNIEnv<'local>,
        activity: &JObject<'_>,
        expected: &AndroidUsbAttachment,
    ) -> jni::errors::Result<Option<JObject<'local>>> {
        let iterator = usb_device_iterator(env, activity)?;
        while env.call_method(&iterator, "hasNext", "()Z", &[])?.z()? {
            let device = env.call_method(&iterator, "next", "()Ljava/lang/Object;", &[])?.l()?;
            if usb_attachment(env, &device)? == *expected {
                return Ok(Some(device));
            }
        }
        Ok(None)
    }

    fn usb_device_iterator<'local>(
        env: &mut JNIEnv<'local>,
        activity: &JObject<'_>,
    ) -> jni::errors::Result<JObject<'local>> {
        let service = env.new_string("usb")?;
        let manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service)],
            )?
            .l()?;
        let devices =
            env.call_method(&manager, "getDeviceList", "()Ljava/util/HashMap;", &[])?.l()?;
        let values = env.call_method(&devices, "values", "()Ljava/util/Collection;", &[])?.l()?;
        env.call_method(&values, "iterator", "()Ljava/util/Iterator;", &[])?.l()
    }

    fn usb_attachment(
        env: &mut JNIEnv<'_>,
        device: &JObject<'_>,
    ) -> jni::errors::Result<AndroidUsbAttachment> {
        let name = env.call_method(device, "getDeviceName", "()Ljava/lang/String;", &[])?.l()?;
        let name = env.get_string(&JString::from(name))?.to_string_lossy().into_owned();
        Ok(AndroidUsbAttachment {
            device_id: env.call_method(device, "getDeviceId", "()I", &[])?.i()?,
            vendor_id: env.call_method(device, "getVendorId", "()I", &[])?.i()?,
            product_id: env.call_method(device, "getProductId", "()I", &[])?.i()?,
            device_name: name,
        })
    }

    fn usb_has_permission(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        device: &JObject<'_>,
    ) -> jni::errors::Result<bool> {
        let service = env.new_string("usb")?;
        let manager = env
            .call_method(
                activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::Object(&service)],
            )?
            .l()?;
        env.call_method(
            &manager,
            "hasPermission",
            "(Landroid/hardware/usb/UsbDevice;)Z",
            &[JValue::Object(device)],
        )?
        .z()
    }

    fn usb_was_requested(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        attachment: &AndroidUsbAttachment,
    ) -> jni::errors::Result<bool> {
        permission_was_requested(env, activity, &usb_marker(attachment))
    }

    fn mark_usb_requested(
        env: &mut JNIEnv<'_>,
        activity: &JObject<'_>,
        attachment: &AndroidUsbAttachment,
    ) -> jni::errors::Result<()> {
        let marker = usb_marker(attachment);
        mark_permissions_requested(env, activity, &[&marker])
    }

    fn usb_marker(attachment: &AndroidUsbAttachment) -> String {
        format!(
            "usb:{}:{}:{}:{}",
            attachment.device_id,
            attachment.vendor_id,
            attachment.product_id,
            attachment.device_name
        )
    }

    fn has_window_focus(env: &mut JNIEnv<'_>, activity: &JObject<'_>) -> jni::errors::Result<bool> {
        env.call_method(activity, "hasWindowFocus", "()Z", &[])?.z()
    }

    fn jni_int(value: usize) -> jni::errors::Result<i32> {
        i32::try_from(value).map_err(|_| invalid_arguments())
    }

    fn invalid_arguments() -> jni::errors::Error {
        jni::errors::Error::JniCall(jni::errors::JniError::InvalidArguments)
    }

    fn failure(code: &str, retryable: bool) -> PlatformFailure {
        PlatformFailure { code: code.into(), retryable }
    }
}

#[cfg(target_os = "ios")]
mod native_platform {
    use std::sync::Mutex;

    use block2::RcBlock;
    use objc2::{MainThreadMarker, runtime::Bool};
    use objc2_foundation::NSError;
    use objc2_ui_kit::UIApplication;
    use objc2_user_notifications::{UNAuthorizationOptions, UNUserNotificationCenter};
    use styrene_ui_apple_bridge::NativeAuthorization;
    use styrene_ui_platform::{
        AndroidUsbAttachment, AuthorizationState, CandidatePayload, MAX_CANDIDATE_PAYLOAD_BYTES,
        PermissionKind, PermissionStatus, PlatformFailure, PlatformSnapshot,
        TextAcquisitionCompletion, TextAcquisitionFailure, TextAcquisitionGeneration,
    };

    static CONFIGURATION_SENDER: Mutex<Option<async_channel::Sender<()>>> = Mutex::new(None);

    #[allow(clippy::unnecessary_wraps)]
    pub fn prepare_subscription() -> Option<async_channel::Receiver<()>> {
        let (sender, receiver) = async_channel::bounded(1);
        if let Ok(mut current) = CONFIGURATION_SENDER.lock() {
            *current = Some(sender);
        }
        styrene_ui_apple_bridge::install_content_size_observer(|| {
            if let Ok(current) = CONFIGURATION_SENDER.lock()
                && let Some(sender) = current.as_ref()
            {
                let _ = sender.try_send(());
            }
        });
        Some(receiver)
    }

    pub async fn enrich_snapshot(snapshot: &mut PlatformSnapshot) {
        snapshot.permissions = vec![
            PermissionStatus {
                kind: PermissionKind::Bluetooth,
                state: authorization(styrene_ui_apple_bridge::bluetooth_authorization()),
            },
            PermissionStatus {
                kind: PermissionKind::Camera,
                state: authorization(styrene_ui_apple_bridge::camera_authorization()),
            },
            PermissionStatus { kind: PermissionKind::Usb, state: AuthorizationState::Unavailable },
        ];
        snapshot.notification_authorization =
            query_notification_authorization().await.unwrap_or(AuthorizationState::Unavailable);
        let Some(marker) = MainThreadMarker::new() else {
            snapshot.accessibility.text_scale = styrene_ui_platform::TextScale::Unavailable;
            return;
        };
        let application = UIApplication::sharedApplication(marker);
        let category = application.preferredContentSizeCategory().to_string();
        snapshot.accessibility.text_scale = super::ios_text_scale_category(&category);
    }

    pub async fn request_permission(
        kind: PermissionKind,
    ) -> Result<PermissionStatus, PlatformFailure> {
        let state = match kind {
            PermissionKind::Camera => request_camera().await?,
            PermissionKind::Bluetooth => request_bluetooth().await?,
            PermissionKind::Usb => AuthorizationState::Unavailable,
        };
        Ok(PermissionStatus { kind, state })
    }

    pub async fn request_notifications() -> Result<AuthorizationState, PlatformFailure> {
        let (sender, receiver) = async_channel::bounded(1);
        let callback = move |granted: Bool, error: *mut NSError| {
            let result = if error.is_null() {
                Ok(if granted.as_bool() {
                    AuthorizationState::Granted
                } else {
                    AuthorizationState::Denied
                })
            } else {
                Err(failure("ios_notification_request_failed", true))
            };
            let _ = sender.try_send(result);
        };
        require_send_sync(&callback);
        let completion = RcBlock::new(callback);
        let options = UNAuthorizationOptions::Alert
            | UNAuthorizationOptions::Sound
            | UNAuthorizationOptions::Badge;
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(options, &completion);
        tokio::time::timeout(std::time::Duration::from_mins(2), receiver.recv())
            .await
            .map_err(|_| failure("ios_notification_callback_timeout", true))?
            .map_err(|_| failure("ios_notification_callback_closed", true))??;
        query_notification_authorization().await
    }

    pub async fn open_application_settings() -> Result<(), PlatformFailure> {
        if styrene_ui_apple_bridge::open_application_settings()
            .map_err(|_| failure("ios_settings_open_failed", true))?
        {
            Ok(())
        } else {
            Err(failure("ios_settings_open_failed", true))
        }
    }

    #[allow(dead_code)] // Consumed by the compose integration in the adjacent workflow task.
    pub fn read_clipboard_text(
        generation: TextAcquisitionGeneration,
    ) -> std::future::Ready<TextAcquisitionCompletion> {
        let result = match styrene_ui_apple_bridge::clipboard_text(MAX_CANDIDATE_PAYLOAD_BYTES) {
            Ok(Some(value)) => CandidatePayload::from_service_bytes(value).map_err(Into::into),
            Ok(None) => Err(TextAcquisitionFailure::Unavailable),
            Err(styrene_ui_apple_bridge::NativeBridgeFailure::Oversized) => {
                Err(TextAcquisitionFailure::Oversized)
            }
            Err(_) => Err(TextAcquisitionFailure::Unavailable),
        };
        std::future::ready(TextAcquisitionCompletion { generation, result })
    }

    pub fn write_clipboard_text(value: String) -> std::future::Ready<Result<(), PlatformFailure>> {
        std::future::ready(
            styrene_ui_apple_bridge::set_clipboard_text(&value)
                .map_err(|_| failure("ios_clipboard_write_failed", true)),
        )
    }

    pub fn android_usb_attachments()
    -> std::future::Ready<Result<Vec<AndroidUsbAttachment>, PlatformFailure>> {
        std::future::ready(Ok(Vec::new()))
    }

    pub fn request_android_usb_authorization(
        _: AndroidUsbAttachment,
    ) -> std::future::Ready<Result<AuthorizationState, PlatformFailure>> {
        std::future::ready(Ok(AuthorizationState::Unavailable))
    }

    async fn request_camera() -> Result<AuthorizationState, PlatformFailure> {
        let current = authorization(styrene_ui_apple_bridge::camera_authorization());
        if current != AuthorizationState::NotDetermined {
            return Ok(current);
        }
        let (sender, receiver) = async_channel::bounded(1);
        styrene_ui_apple_bridge::request_camera(move |state| {
            let _ = sender.try_send(authorization(state));
        })
        .map_err(|_| failure("ios_camera_request_unavailable", false))?;
        tokio::time::timeout(std::time::Duration::from_mins(2), receiver.recv())
            .await
            .map_err(|_| failure("ios_camera_callback_timeout", true))?
            .map_err(|_| failure("ios_camera_callback_closed", true))
    }

    async fn request_bluetooth() -> Result<AuthorizationState, PlatformFailure> {
        let current = authorization(styrene_ui_apple_bridge::bluetooth_authorization());
        if current != AuthorizationState::NotDetermined {
            return Ok(current);
        }
        let (sender, receiver) = async_channel::bounded(1);
        let request = styrene_ui_apple_bridge::request_bluetooth(move |state| {
            let _ = sender.try_send(authorization(state));
        })
        .map_err(|_| failure("ios_bluetooth_request_unavailable", false))?;
        let result = tokio::time::timeout(std::time::Duration::from_mins(2), receiver.recv())
            .await
            .map_err(|_| failure("ios_bluetooth_callback_timeout", true))?
            .map_err(|_| failure("ios_bluetooth_callback_closed", true));
        drop(request);
        result
    }

    async fn query_notification_authorization() -> Result<AuthorizationState, PlatformFailure> {
        let (sender, receiver) = async_channel::bounded(1);
        styrene_ui_apple_bridge::query_notification_authorization(move |state| {
            let _ = sender.try_send(authorization(state));
        });
        tokio::time::timeout(std::time::Duration::from_secs(10), receiver.recv())
            .await
            .map_err(|_| failure("ios_notification_snapshot_timeout", true))?
            .map_err(|_| failure("ios_notification_snapshot_closed", true))
    }

    const fn authorization(state: NativeAuthorization) -> AuthorizationState {
        match state {
            NativeAuthorization::NotDetermined => AuthorizationState::NotDetermined,
            NativeAuthorization::Granted => AuthorizationState::Granted,
            NativeAuthorization::Denied => AuthorizationState::Denied,
            NativeAuthorization::Restricted => AuthorizationState::Restricted,
            NativeAuthorization::Unavailable => AuthorizationState::Unavailable,
        }
    }

    fn require_send_sync<T: Send + Sync>(_: &T) {}

    fn failure(code: &str, retryable: bool) -> PlatformFailure {
        PlatformFailure { code: code.into(), retryable }
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod native_platform {
    use styrene_ui_platform::{
        AndroidUsbAttachment, AuthorizationState, PermissionKind, PermissionStatus,
        PlatformFailure, PlatformSnapshot, TextAcquisitionCompletion, TextAcquisitionFailure,
        TextAcquisitionGeneration,
    };

    pub fn prepare_subscription() -> Option<async_channel::Receiver<()>> {
        None
    }

    pub fn enrich_snapshot(_: &mut PlatformSnapshot) -> std::future::Ready<()> {
        std::future::ready(())
    }

    pub fn request_permission(
        kind: PermissionKind,
    ) -> std::future::Ready<Result<PermissionStatus, PlatformFailure>> {
        std::future::ready(Ok(PermissionStatus { kind, state: AuthorizationState::Unavailable }))
    }

    pub fn request_notifications() -> std::future::Ready<Result<AuthorizationState, PlatformFailure>>
    {
        std::future::ready(Ok(AuthorizationState::Unavailable))
    }

    pub fn open_application_settings() -> std::future::Ready<Result<(), PlatformFailure>> {
        std::future::ready(Err(PlatformFailure {
            code: "application_settings_unavailable".into(),
            retryable: false,
        }))
    }

    #[allow(dead_code)] // Consumed by the compose integration in the adjacent workflow task.
    pub fn read_clipboard_text(
        generation: TextAcquisitionGeneration,
    ) -> std::future::Ready<TextAcquisitionCompletion> {
        std::future::ready(TextAcquisitionCompletion {
            generation,
            result: Err(TextAcquisitionFailure::Unavailable),
        })
    }

    pub fn write_clipboard_text(_: String) -> std::future::Ready<Result<(), PlatformFailure>> {
        std::future::ready(Err(PlatformFailure {
            code: "clipboard_write_unavailable".into(),
            retryable: false,
        }))
    }

    pub fn android_usb_attachments()
    -> std::future::Ready<Result<Vec<AndroidUsbAttachment>, PlatformFailure>> {
        std::future::ready(Ok(Vec::new()))
    }

    pub fn request_android_usb_authorization(
        _: AndroidUsbAttachment,
    ) -> std::future::Ready<Result<AuthorizationState, PlatformFailure>> {
        std::future::ready(Ok(AuthorizationState::Unavailable))
    }
}

const BACK_LISTENER: &str = r#"
if (!history.state?.styrenePane) {
    history.replaceState({ styrenePane: "root" }, "", location.href);
}
window.addEventListener("popstate", () => {
    document.getElementById("mobile.platform-back")?.click();
});
"#;

const PLATFORM_SNAPSHOT: &str = r#"
const adapter = window.__styrenePlatformAdapter;
if (!adapter) {
    throw new Error("platform adapter is not subscribed");
}
return adapter.snapshot();
"#;

const PLATFORM_SUBSCRIPTION: &str = r#"
window.__styrenePlatformAdapter?.controller.abort();

const controller = new AbortController();
const signal = controller.signal;
const generation = (window.__styrenePlatformGeneration ?? 0) + 1;
window.__styrenePlatformGeneration = generation;
let sequence = 0;
let inFlight = false;
let pendingKind = null;
let droppedEvents = 0;
let viewportBaseline = {
    width: window.visualViewport?.width ?? window.innerWidth,
    height: window.visualViewport?.height ?? window.innerHeight,
    scale: window.visualViewport?.scale ?? 1,
};

const media = {
    dark: matchMedia("(prefers-color-scheme: dark)"),
    contrast: matchMedia("(prefers-contrast: more)"),
    reducedMotion: matchMedia("(prefers-reduced-motion: reduce)"),
};

const editableFocused = () => {
    const active = document.activeElement;
    return active instanceof HTMLInputElement
        || active instanceof HTMLTextAreaElement
        || active instanceof HTMLSelectElement
        || active?.isContentEditable === true;
};

const lifecycle = () => {
    if (document.visibilityState === "hidden") return "background";
    return "active";
};

const snapshot = () => {
    const viewportWidth = window.visualViewport?.width ?? window.innerWidth;
    const viewportHeight = window.visualViewport?.height ?? window.innerHeight;
    const viewportScale = window.visualViewport?.scale ?? 1;
    if (!editableFocused()
        || Math.abs(viewportBaseline.width - viewportWidth) > 40
        || Math.abs(viewportBaseline.scale - viewportScale) > 0.05) {
        viewportBaseline = {
            width: viewportWidth,
            height: viewportHeight,
            scale: viewportScale,
        };
    }
    return {
        generation,
        sequence,
        widthCssPx: Math.max(0, Math.round(window.innerWidth)),
        heightCssPx: Math.max(0, Math.round(window.innerHeight)),
        wide: matchMedia("(min-width: 52rem)").matches,
        appearance: media.dark.matches ? "dark" : "light",
        contrast: media.contrast.matches ? "increased" : "standard",
        motion: media.reducedMotion.matches ? "reduced" : "full",
        textScalePercent: null,
        lifecycle: lifecycle(),
        keyboardVisible: lifecycle() === "active"
            && editableFocused()
            && viewportBaseline.height - viewportHeight > 80,
    };
};

const publish = async (kind) => {
    if (inFlight) {
        droppedEvents += 1;
        pendingKind = pendingKind === null || pendingKind === kind ? kind : "resync";
        return;
    }

    inFlight = true;
    let nextKind = kind;
    do {
        sequence += 1;
        const message = {
            kind: nextKind,
            droppedEvents,
            snapshot: snapshot(),
        };
        droppedEvents = 0;
        pendingKind = null;
        dioxus.send(message);
        await dioxus.recv();
        nextKind = pendingKind;
    } while (!signal.aborted && nextKind !== null);
    inFlight = false;
};

const listen = (target, event, kind) => {
    target.addEventListener(event, () => publish(kind), { signal });
};

listen(window, "resize", "resync");
listen(document, "visibilitychange", "resync");
listen(document, "focusin", "geometry");
listen(document, "focusout", "geometry");
if (window.visualViewport) {
    listen(window.visualViewport, "resize", "geometry");
    listen(window.visualViewport, "scroll", "geometry");
}
for (const preference of Object.values(media)) {
    listen(preference, "change", "accessibility");
}

window.__styrenePlatformAdapter = { controller, generation, snapshot };
await publish("resync");
await new Promise((resolve) => signal.addEventListener("abort", resolve, { once: true }));
sequence += 1;
dioxus.send({
    kind: "closed",
    droppedEvents: 0,
    snapshot: snapshot(),
});
await dioxus.recv();
"#;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct WebPlatformSnapshot {
    generation: u64,
    sequence: u64,
    width_css_px: u32,
    height_css_px: u32,
    wide: bool,
    appearance: WebAppearance,
    contrast: WebContrast,
    motion: WebMotion,
    text_scale_percent: Option<u16>,
    lifecycle: WebLifecycle,
    keyboard_visible: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebAppearance {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebContrast {
    Standard,
    Increased,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebMotion {
    Full,
    Reduced,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebLifecycle {
    Active,
    Inactive,
    Background,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebPlatformMessage {
    kind: WebEventKind,
    dropped_events: u64,
    snapshot: WebPlatformSnapshot,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum WebEventKind {
    Window,
    Accessibility,
    Geometry,
    Lifecycle,
    Resync,
    Closed,
}

#[derive(Clone, Copy, Debug, Default)]
struct WebViewPlatformService;

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)] // Consumed by the compose integration in the adjacent workflow task.
pub struct NativeClipboardTextReader;

impl ClipboardTextReader for NativeClipboardTextReader {
    fn read_clipboard_text(
        &self,
        generation: TextAcquisitionGeneration,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion> {
        Box::pin(async move { native_platform::read_clipboard_text(generation).await })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeClipboardTextWriter;

impl ClipboardTextWriter for NativeClipboardTextWriter {
    fn write_clipboard_text(
        &self,
        value: String,
    ) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(async move { native_platform::write_clipboard_text(value).await })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeApplicationSettingsService;

impl styrene_ui_platform::ApplicationSettingsService for NativeApplicationSettingsService {
    fn open_application_settings(&self) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(async move { native_platform::open_application_settings().await })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeOpaqueDocumentSharer;

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeOpaqueDocumentPicker;

impl OpaqueDocumentPicker for NativeOpaqueDocumentPicker {
    fn pick_document(
        &self,
        generation: DocumentRequestGeneration,
    ) -> PlatformFuture<'_, DocumentPickerCompletion> {
        Box::pin(async move {
            #[cfg(target_os = "ios")]
            let result = {
                let (sender, receiver) = async_channel::bounded(1);
                match styrene_ui_apple_bridge::present_identity_backup_picker(
                    styrene_ui_platform::MAX_OPAQUE_DOCUMENT_BYTES,
                    move |result| {
                        let result = result
                            .map_err(|failure| match failure {
                                styrene_ui_apple_bridge::NativeDocumentPickerFailure::Cancelled => {
                                    DocumentPickerFailure::Cancelled
                                }
                                styrene_ui_apple_bridge::NativeDocumentPickerFailure::Oversized => {
                                    DocumentPickerFailure::Oversized
                                }
                                styrene_ui_apple_bridge::NativeDocumentPickerFailure::ReadFailed => {
                                    DocumentPickerFailure::ReadFailed
                                }
                                styrene_ui_apple_bridge::NativeDocumentPickerFailure::PresentationUnavailable => {
                                    DocumentPickerFailure::Unavailable
                                }
                            })
                            .and_then(|bytes| OpaqueDocument::new(bytes).map_err(Into::into));
                        let _ = sender.try_send(result);
                    },
                ) {
                    Ok(request) => {
                        let received = tokio::time::timeout(
                            std::time::Duration::from_mins(5),
                            receiver.recv(),
                        )
                        .await;
                        drop(request);
                        match received {
                            Ok(Ok(result)) => result,
                            Ok(Err(_)) | Err(_) => Err(DocumentPickerFailure::ReadFailed),
                        }
                    }
                    Err(_) => Err(DocumentPickerFailure::Unavailable),
                }
            };
            #[cfg(target_os = "android")]
            let result = native_platform::pick_identity_backup()
                .await
                .and_then(|bytes| OpaqueDocument::new(bytes).map_err(Into::into));
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            let result = Err(styrene_ui_platform::DocumentPickerFailure::Unavailable);
            DocumentPickerCompletion { generation, result }
        })
    }
}

impl OpaqueDocumentSharer for NativeOpaqueDocumentSharer {
    fn present_document_share(
        &self,
        generation: DocumentRequestGeneration,
        document: OpaqueDocument,
    ) -> PlatformFuture<'_, DocumentShareCompletion> {
        Box::pin(async move {
            #[cfg(target_os = "ios")]
            let result = {
                let map_error = |error| match error {
                    styrene_ui_apple_bridge::NativeBridgeFailure::MainThreadUnavailable
                    | styrene_ui_apple_bridge::NativeBridgeFailure::PresentationUnavailable => {
                        DocumentShareFailure::Unavailable
                    }
                    styrene_ui_apple_bridge::NativeBridgeFailure::MediaTypeUnavailable
                    | styrene_ui_apple_bridge::NativeBridgeFailure::Oversized
                    | styrene_ui_apple_bridge::NativeBridgeFailure::WriteFailed => {
                        DocumentShareFailure::PresentationFailed
                    }
                };
                let (presented, presentation) = async_channel::bounded(1);
                match styrene_ui_apple_bridge::present_identity_backup(
                    document.as_bytes(),
                    move || {
                        let _ = presented.force_send(());
                    },
                ) {
                    Ok(()) => match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        presentation.recv(),
                    )
                    .await
                    {
                        Ok(Ok(())) => Ok(styrene_ui_platform::DocumentShareOutcome::Presented),
                        Ok(Err(_)) | Err(_) => {
                            let _ = styrene_ui_apple_bridge::remove_identity_backup_temp_file();
                            Err(DocumentShareFailure::PresentationFailed)
                        }
                    },
                    Err(error) => Err(map_error(error)),
                }
            };
            #[cfg(target_os = "android")]
            let result = native_platform::share_identity_backup(document.into_bytes()).await;
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            let result = {
                let _ = document;
                Err(DocumentShareFailure::Unavailable)
            };
            DocumentShareCompletion { generation, result }
        })
    }
}

struct WebViewPlatformEventStream {
    eval: document::Eval,
    native: Option<async_channel::Receiver<()>>,
    generation: Option<u64>,
}

impl WebPlatformSnapshot {
    fn platform_snapshot(self) -> PlatformSnapshot {
        PlatformSnapshot {
            generation: self.generation,
            sequence: self.sequence,
            window: self.window(),
            accessibility: self.accessibility(),
            geometry: self.geometry(),
            lifecycle: self.lifecycle.into(),
            permissions: Vec::new(),
            notification_authorization: AuthorizationState::Unavailable,
        }
    }

    const fn window(self) -> WindowMetrics {
        WindowMetrics {
            class: if self.wide { WindowClass::Wide } else { WindowClass::Compact },
            width_css_px: self.width_css_px,
            height_css_px: self.height_css_px,
        }
    }

    const fn accessibility(self) -> AccessibilityPreferences {
        AccessibilityPreferences {
            text_scale: match self.text_scale_percent {
                Some(percent) => TextScale::Percent(percent),
                None => TextScale::Unavailable,
            },
            appearance: match self.appearance {
                WebAppearance::Light => Appearance::Light,
                WebAppearance::Dark => Appearance::Dark,
            },
            contrast: match self.contrast {
                WebContrast::Standard => Contrast::Standard,
                WebContrast::Increased => Contrast::Increased,
            },
            motion: match self.motion {
                WebMotion::Full => MotionPreference::Full,
                WebMotion::Reduced => MotionPreference::Reduced,
            },
        }
    }

    const fn geometry(self) -> PlatformGeometry {
        PlatformGeometry {
            insets: PlatformInsets::CssEnvironment,
            keyboard: KeyboardGeometry::WebViewManaged { visible: self.keyboard_visible },
        }
    }
}

impl From<WebLifecycle> for ApplicationLifecycle {
    fn from(value: WebLifecycle) -> Self {
        match value {
            WebLifecycle::Active => Self::Active,
            WebLifecycle::Inactive => Self::Inactive,
            WebLifecycle::Background => Self::Background,
        }
    }
}

impl WebPlatformMessage {
    fn platform_event(self) -> Option<PlatformEvent> {
        if self.kind == WebEventKind::Closed {
            return None;
        }
        let generation = self.snapshot.generation;
        if self.dropped_events > 0
            || matches!(self.kind, WebEventKind::Accessibility | WebEventKind::Resync)
        {
            return Some(PlatformEvent::ResyncRequired {
                generation,
                dropped_events: self.dropped_events,
            });
        }

        let sequence = self.snapshot.sequence;
        let change = match self.kind {
            WebEventKind::Window => PlatformChange::Window(self.snapshot.window()),
            WebEventKind::Accessibility => {
                PlatformChange::Accessibility(self.snapshot.accessibility())
            }
            WebEventKind::Geometry => PlatformChange::Geometry(self.snapshot.geometry()),
            WebEventKind::Lifecycle => PlatformChange::Lifecycle(self.snapshot.lifecycle.into()),
            WebEventKind::Resync => unreachable!("resync events return before projection"),
            WebEventKind::Closed => unreachable!("closed events return before projection"),
        };
        Some(PlatformEvent::Changed { generation, sequence, change })
    }
}

impl PlatformEventStream for WebViewPlatformEventStream {
    fn next(&mut self) -> PlatformFuture<'_, Option<PlatformEvent>> {
        Box::pin(async move {
            enum Source<T> {
                Web(T),
                Native,
                NativeClosed,
            }

            loop {
                let message = if let Some(native) = self.native.clone() {
                    let source = {
                        let web = self.eval.recv::<WebPlatformMessage>();
                        let native = native.recv();
                        let mut web = std::pin::pin!(web);
                        let mut native = std::pin::pin!(native);
                        std::future::poll_fn(|context| {
                            use std::future::Future;
                            use std::task::Poll;

                            if let Poll::Ready(message) = web.as_mut().poll(context) {
                                return Poll::Ready(Source::Web(message));
                            }
                            match native.as_mut().poll(context) {
                                Poll::Ready(Ok(())) => Poll::Ready(Source::Native),
                                Poll::Ready(Err(_)) => Poll::Ready(Source::NativeClosed),
                                Poll::Pending => Poll::Pending,
                            }
                        })
                        .await
                    };
                    match source {
                        Source::Web(message) => message,
                        Source::Native => {
                            if let Some(event) = native_resync(self.generation) {
                                return Some(event);
                            }
                            continue;
                        }
                        Source::NativeClosed => {
                            self.native = None;
                            continue;
                        }
                    }
                } else {
                    self.eval.recv::<WebPlatformMessage>().await
                };
                let _ = self.eval.send(());
                let message = message.ok()?;
                self.generation = Some(message.snapshot.generation);
                if let Some(event) = message.platform_event() {
                    return Some(event);
                }
            }
        })
    }
}

fn native_resync(generation: Option<u64>) -> Option<PlatformEvent> {
    generation.map(|generation| PlatformEvent::ResyncRequired { generation, dropped_events: 0 })
}

impl PlatformService for WebViewPlatformService {
    fn snapshot(&self) -> PlatformFuture<'_, Result<PlatformSnapshot, PlatformFailure>> {
        Box::pin(async move {
            let mut snapshot = document::eval(PLATFORM_SNAPSHOT)
                .join::<WebPlatformSnapshot>()
                .await
                .map(WebPlatformSnapshot::platform_snapshot)
                .map_err(|error| platform_eval_failure(&error))?;
            native_platform::enrich_snapshot(&mut snapshot).await;
            Ok(snapshot)
        })
    }

    fn subscribe(&self) -> Result<Box<dyn PlatformEventStream>, PlatformFailure> {
        Ok(Box::new(WebViewPlatformEventStream {
            eval: document::eval(PLATFORM_SUBSCRIPTION),
            native: native_platform::prepare_subscription(),
            generation: None,
        }))
    }

    fn request_permission(
        &self,
        kind: PermissionKind,
    ) -> PlatformFuture<'_, Result<PermissionStatus, PlatformFailure>> {
        Box::pin(async move { native_platform::request_permission(kind).await })
    }

    fn request_notification_authorization(
        &self,
    ) -> PlatformFuture<'_, Result<AuthorizationState, PlatformFailure>> {
        Box::pin(async move { native_platform::request_notifications().await })
    }

    fn android_usb_attachments(
        &self,
    ) -> PlatformFuture<'_, Result<Vec<AndroidUsbAttachment>, PlatformFailure>> {
        Box::pin(async move { native_platform::android_usb_attachments().await })
    }

    fn request_android_usb_authorization(
        &self,
        attachment: AndroidUsbAttachment,
    ) -> PlatformFuture<'_, Result<AuthorizationState, PlatformFailure>> {
        Box::pin(
            async move { native_platform::request_android_usb_authorization(attachment).await },
        )
    }
}

fn platform_eval_failure(error: &document::EvalError) -> PlatformFailure {
    let (code, retryable) = match error {
        document::EvalError::Finished | document::EvalError::Communication(_) => {
            ("platform_eval_interrupted", true)
        }
        document::EvalError::Unsupported => ("platform_eval_unsupported", false),
        document::EvalError::InvalidJs(_) => ("platform_eval_invalid", false),
        document::EvalError::Serialization(_) => ("platform_eval_incompatible", false),
        _ => ("platform_eval_failed", false),
    };
    PlatformFailure { code: code.into(), retryable }
}

pub fn use_back_navigation() {
    use_effect(move || {
        document::eval(BACK_LISTENER);
    });
}

pub fn use_platform_snapshot() -> Signal<Option<PlatformSnapshot>> {
    let mut snapshot = use_signal(|| None);
    use_drop(move || {
        document::eval("window.__styrenePlatformAdapter?.controller.abort();");
    });
    use_effect(move || {
        spawn(async move {
            loop {
                run_platform_subscription(&mut snapshot).await;
                let _ = document::eval("await new Promise((resolve) => setTimeout(resolve, 250));")
                    .await;
            }
        });
    });
    snapshot
}

#[cfg(not(feature = "ui-test"))]
pub async fn android_usb_attachments() -> Result<Vec<AndroidUsbAttachment>, PlatformFailure> {
    WebViewPlatformService.android_usb_attachments().await
}

#[cfg(all(target_os = "ios", not(feature = "ui-test")))]
pub async fn request_permission(kind: PermissionKind) -> Result<PermissionStatus, PlatformFailure> {
    WebViewPlatformService.request_permission(kind).await
}

#[cfg(all(target_os = "android", not(feature = "ui-test")))]
pub async fn native_android_usb_attachments() -> Result<Vec<AndroidUsbAttachment>, PlatformFailure>
{
    native_platform::android_usb_attachments().await
}

#[cfg(not(feature = "ui-test"))]
pub async fn request_android_usb_authorization(
    attachment: AndroidUsbAttachment,
) -> Result<AuthorizationState, PlatformFailure> {
    WebViewPlatformService.request_android_usb_authorization(attachment).await
}

async fn run_platform_subscription(snapshot: &mut Signal<Option<PlatformSnapshot>>) {
    let service = WebViewPlatformService;
    let Ok(mut events) = service.subscribe() else {
        return;
    };
    let mut state = None::<PlatformState>;
    let mut resync_required = true;
    while let Some(event) = events.next().await {
        apply_platform_event(&mut state, &mut resync_required, event);

        if resync_required {
            let Ok(current) = service.snapshot().await else {
                continue;
            };
            if let Some(state) = state.as_mut() {
                if state.replace_resynced_snapshot(current) == PlatformApplyResult::IgnoredStale {
                    continue;
                }
            } else {
                state = Some(PlatformState::new(current));
            }
            resync_required = false;
        }

        if let Some(state) = &state {
            snapshot.set(Some(state.snapshot().clone()));
        }
    }
}

fn apply_platform_event(
    state: &mut Option<PlatformState>,
    resync_required: &mut bool,
    event: PlatformEvent,
) {
    if *resync_required {
        return;
    }
    let Some(state) = state.as_mut() else {
        *resync_required = true;
        return;
    };
    if state.apply_event(event) == PlatformApplyResult::ResyncRequired {
        *resync_required = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listener_uses_history_state_and_fixed_dioxus_action() {
        assert!(BACK_LISTENER.contains("history.replaceState"));
        assert!(BACK_LISTENER.contains("popstate"));
        assert!(BACK_LISTENER.contains("mobile.platform-back"));
        assert!(!BACK_LISTENER.contains("window.ipc"));
    }

    #[test]
    fn android_policy_preserves_native_scale_and_api_permission_boundaries() {
        use android_policy::{
            BLUETOOTH_CONNECT, BLUETOOTH_SCAN, CAMERA, LOCATION, POST_NOTIFICATIONS,
            font_scale_percent, merge_authorization, notification_permission_names,
            permission_names,
        };

        assert_eq!(font_scale_percent(1.0), Some(100));
        assert_eq!(font_scale_percent(1.3), Some(130));
        assert_eq!(font_scale_percent(2.0), Some(200));
        assert_eq!(font_scale_percent(f32::NAN), None);
        assert_eq!(font_scale_percent(f32::INFINITY), None);
        assert_eq!(font_scale_percent(0.0), None);
        assert_eq!(permission_names(PermissionKind::Bluetooth, 30), &[LOCATION]);
        assert_eq!(
            permission_names(PermissionKind::Bluetooth, 31),
            &[BLUETOOTH_SCAN, BLUETOOTH_CONNECT]
        );
        assert_eq!(permission_names(PermissionKind::Camera, 35), &[CAMERA]);
        assert!(permission_names(PermissionKind::Usb, 35).is_empty());
        assert!(notification_permission_names(32).is_empty());
        assert_eq!(notification_permission_names(33), &[POST_NOTIFICATIONS]);
        assert_eq!(
            merge_authorization(AuthorizationState::Granted, AuthorizationState::NotDetermined),
            AuthorizationState::NotDetermined
        );
        assert_eq!(
            merge_authorization(AuthorizationState::NotDetermined, AuthorizationState::Denied),
            AuthorizationState::Denied
        );
        assert_eq!(
            merge_authorization(AuthorizationState::Denied, AuthorizationState::Restricted),
            AuthorizationState::Restricted
        );
    }

    #[test]
    fn android_document_share_statuses_are_typed() {
        use styrene_ui_platform::{DocumentShareFailure, DocumentShareOutcome};

        assert_eq!(android_policy::document_share_result(0), Ok(DocumentShareOutcome::Presented));
        assert_eq!(
            android_policy::document_share_result(1),
            Err(DocumentShareFailure::Unavailable)
        );
        assert_eq!(
            android_policy::document_share_result(2),
            Err(DocumentShareFailure::PresentationFailed)
        );
    }

    #[test]
    fn ios_dynamic_type_values_remain_named_categories() {
        use styrene_ui_platform::TextScaleCategory;

        assert_eq!(
            ios_text_scale_category("UICTContentSizeCategoryL"),
            TextScale::Category(TextScaleCategory::Large)
        );
        assert_eq!(
            ios_text_scale_category("UICTContentSizeCategoryAccessibilityXXXL"),
            TextScale::Category(TextScaleCategory::AccessibilityExtraExtraExtraLarge)
        );
        assert_eq!(
            ios_text_scale_category("UICTContentSizeCategoryFuture"),
            TextScale::Category(TextScaleCategory::Unknown)
        );
        assert_eq!(
            ios_text_scale_category("UICTContentSizeCategoryUnspecified"),
            TextScale::Unavailable
        );
    }

    fn web_snapshot(generation: u64, sequence: u64) -> WebPlatformSnapshot {
        WebPlatformSnapshot {
            generation,
            sequence,
            width_css_px: 390,
            height_css_px: 844,
            wide: false,
            appearance: WebAppearance::Dark,
            contrast: WebContrast::Increased,
            motion: WebMotion::Reduced,
            text_scale_percent: Some(200),
            lifecycle: WebLifecycle::Active,
            keyboard_visible: true,
        }
    }

    #[test]
    fn snapshot_maps_webview_facts_to_typed_platform_state() {
        let snapshot = web_snapshot(4, 7).platform_snapshot();

        assert_eq!(snapshot.generation, 4);
        assert_eq!(snapshot.window.class, WindowClass::Compact);
        assert_eq!(snapshot.accessibility.appearance, Appearance::Dark);
        assert_eq!(snapshot.accessibility.contrast, Contrast::Increased);
        assert_eq!(snapshot.accessibility.text_scale, TextScale::Percent(200));
        assert_eq!(snapshot.geometry.keyboard, KeyboardGeometry::WebViewManaged { visible: true });
        assert_eq!(snapshot.notification_authorization, AuthorizationState::Unavailable);
    }

    #[test]
    fn dropped_callbacks_require_authoritative_resnapshot() {
        let event = WebPlatformMessage {
            kind: WebEventKind::Geometry,
            dropped_events: 3,
            snapshot: web_snapshot(8, 12),
        }
        .platform_event()
        .expect("geometry event");

        assert_eq!(event, PlatformEvent::ResyncRequired { generation: 8, dropped_events: 3 });
    }

    #[test]
    fn native_callbacks_wait_for_an_authoritative_generation() {
        assert_eq!(native_resync(None), None);
        assert_eq!(
            native_resync(Some(12)),
            Some(PlatformEvent::ResyncRequired { generation: 12, dropped_events: 0 })
        );
    }

    #[test]
    fn web_accessibility_callbacks_resnapshot_native_text_scale() {
        let event = WebPlatformMessage {
            kind: WebEventKind::Accessibility,
            dropped_events: 0,
            snapshot: web_snapshot(8, 12),
        }
        .platform_event()
        .expect("accessibility event");

        assert_eq!(event, PlatformEvent::ResyncRequired { generation: 8, dropped_events: 0 });
    }

    #[test]
    fn subscription_is_bounded_acknowledged_and_idempotent() {
        assert!(PLATFORM_SUBSCRIPTION.contains("controller.abort"));
        assert!(PLATFORM_SUBSCRIPTION.contains("inFlight"));
        assert!(PLATFORM_SUBSCRIPTION.contains("droppedEvents"));
        assert!(PLATFORM_SUBSCRIPTION.contains("await dioxus.recv()"));
        assert!(PLATFORM_SUBSCRIPTION.contains("kind: \"closed\""));
        assert!(PLATFORM_SUBSCRIPTION.contains("prefers-color-scheme"));
        assert!(PLATFORM_SUBSCRIPTION.contains("prefers-contrast"));
        assert!(PLATFORM_SUBSCRIPTION.contains("prefers-reduced-motion"));
        assert!(PLATFORM_SUBSCRIPTION.contains("visibilitychange"));
        assert!(PLATFORM_SUBSCRIPTION.contains("visualViewport"));
    }

    #[test]
    fn webview_payload_deserializes_exact_wire_names() {
        let payload = r#"{
            "kind": "accessibility",
            "droppedEvents": 0,
            "snapshot": {
                "generation": 2,
                "sequence": 5,
                "widthCssPx": 390,
                "heightCssPx": 844,
                "wide": false,
                "appearance": "dark",
                "contrast": "increased",
                "motion": "reduced",
                "textScalePercent": null,
                "lifecycle": "active",
                "keyboardVisible": false
            }
        }"#;

        let message: WebPlatformMessage =
            serde_json::from_str(payload).expect("webview payload must deserialize");

        assert_eq!(message.kind, WebEventKind::Accessibility);
        assert_eq!(message.snapshot.text_scale_percent, None);
        assert_eq!(message.snapshot.appearance, WebAppearance::Dark);
    }

    #[test]
    fn failed_resnapshot_blocks_later_partial_events() {
        let initial = web_snapshot(3, 7).platform_snapshot();
        let mut state = Some(PlatformState::new(initial.clone()));
        let mut resync_required = true;

        apply_platform_event(
            &mut state,
            &mut resync_required,
            PlatformEvent::Changed {
                generation: 3,
                sequence: 8,
                change: PlatformChange::Lifecycle(ApplicationLifecycle::Background),
            },
        );

        assert_eq!(state.expect("state").snapshot(), &initial);
        assert!(resync_required);
    }
}
