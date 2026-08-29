# Mobile Support Matrix

Android and iOS packaging advance independently. Both hosts render the same
Rust-owned Dioxus product and consume the same backend contracts.

| Host package | Minimum OS | Build target | Secure identity |
|---|---:|---:|---|
| `styrene-mobile-android` | Android 9 / API 28 | API 35 | Android Keystore-wrapped root secret |
| `styrene-mobile-ios` | iOS 17 | Current installed iOS SDK | Apple Keychain |

Android compatibility is capability-based. API level alone does not establish
support. Packaged validation must record the System WebView version, CPU ABI,
Keystore security level when observable, window behavior, memory class, and
available Bluetooth or USB transports. A missing optional capability must
degrade to typed unavailable state rather than change messaging truth.

The Android validation lanes are API 28, API 35, and a physical commodity
device. The iOS lanes are the current simulator and a physical iPhone. Release
evidence records the host package version, shared UI revision, backend revision,
artifact hash, device OS, and WebView version.
