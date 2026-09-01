fn main() {
    dioxus::LaunchBuilder::mobile()
        .with_cfg(
            dioxus::mobile::Config::new()
                .with_custom_index(styrene_mobile::MOBILE_INDEX.to_string()),
        )
        .launch(styrene_mobile::App);
}

#[cfg(test)]
mod tests {
    const ANDROID_MANIFEST: &str = include_str!("../AndroidManifest.xml");
    const DIOXUS_CONFIG: &str = include_str!("../Dioxus.toml");
    const WRY_FILE_CAPTURE: &str =
        include_str!("../../../vendor/wry/src/android/kotlin/RustWebChromeClient.kt");

    #[test]
    fn packaging_declares_android_compatibility_contract() {
        assert!(DIOXUS_CONFIG.contains("min_sdk = 28"));
        assert!(DIOXUS_CONFIG.contains("target_sdk = 35"));
        assert!(ANDROID_MANIFEST.contains("Theme.AppCompat.DayNight.NoActionBar"));
        assert!(ANDROID_MANIFEST.contains("fontScale"));
        assert!(ANDROID_MANIFEST.contains("uiMode"));
        assert!(ANDROID_MANIFEST.contains("density"));
        assert!(ANDROID_MANIFEST.contains("smallestScreenSize"));
        assert!(ANDROID_MANIFEST.contains("android:stateNotNeeded=\"true\""));
        assert!(ANDROID_MANIFEST.contains("android:allowBackup=\"false\""));
        assert!(ANDROID_MANIFEST.contains("android.permission.CAMERA"));
        assert!(ANDROID_MANIFEST.contains("android.permission.BLUETOOTH_SCAN"));
        assert!(ANDROID_MANIFEST.contains("android.permission.BLUETOOTH_CONNECT"));
        assert!(ANDROID_MANIFEST.contains("android.permission.POST_NOTIFICATIONS"));
        assert!(ANDROID_MANIFEST.contains("android:usesPermissionFlags=\"neverForLocation\""));
        assert!(
            ANDROID_MANIFEST
                .contains("android.permission.ACCESS_FINE_LOCATION\" android:maxSdkVersion=\"30\"")
        );
        assert!(ANDROID_MANIFEST.contains("android.hardware.camera\" android:required=\"false\""));
        assert!(
            ANDROID_MANIFEST.contains("android.hardware.bluetooth_le\" android:required=\"false\"")
        );
        assert!(
            ANDROID_MANIFEST.contains("android.hardware.usb.host\" android:required=\"false\"")
        );
    }

    #[test]
    fn vendored_wry_capture_accepts_bounded_qr_image_types() {
        assert!(WRY_FILE_CAPTURE.contains("fileChooserParams.isCaptureEnabled"));
        assert!(WRY_FILE_CAPTURE.contains("it == \"image/jpeg\" || it == \"image/png\""));
        assert!(WRY_FILE_CAPTURE.contains("MediaStore.ACTION_IMAGE_CAPTURE"));
        assert!(WRY_FILE_CAPTURE.contains("Falling back to default file picker"));
    }
}
