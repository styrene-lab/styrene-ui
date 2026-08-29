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

## WebView Platform Facts

The shared host exposes platform facts through a bounded typed subscription.
Coalesced callbacks require an authoritative snapshot. A new subscription uses
a new generation, and stale callbacks cannot replace the current generation.

| Fact | Current source | Current limit |
|---|---|---|
| Window class and dimensions | Layout viewport | CSS pixel values only |
| Appearance | `prefers-color-scheme` | Limited to values exposed by the WebView |
| Increased contrast | `prefers-contrast` | Android System WebView does not expose this setting reliably |
| Reduced motion | `prefers-reduced-motion` | Limited to values exposed by the WebView |
| Text scale | Android `Configuration.fontScale` and iOS Dynamic Type category | Android reports a percentage. iOS reports a named category without inventing a percentage |
| Lifecycle | Document visibility | The WebView reports active or background, not native inactive state |
| Keyboard | Focus plus visual viewport occlusion | The WebView manages layout resizing |
| Insets | CSS environment variables | Native inset values are not duplicated |
| Android permissions | Runtime permission APIs | Bluetooth scan/connect, camera, and notification gates are queried and requested. USB remains unavailable until the explicit device flow exists |
| iOS permissions | Typed unavailable state | Bluetooth and camera request adapters are not implemented |
| iOS notifications | `UNUserNotificationCenter` request callback | Request results are granted or denied. Snapshot status remains unavailable because the pinned safe binding cannot read notification settings under the workspace unsafe-code prohibition |

Native Android calls use Wry's bounded main-thread pipe and a one-slot result
channel. Dispatch fails closed when no activity or queue capacity is available.
Permission completion is observed after the system dialog returns focus. A
bounded timeout is reported rather than fabricating a decision. Native facts are
re-read for authoritative resnapshots, so stale WebView generations cannot
replace the current platform state.

Android stores a request marker before it opens a permission prompt. This marker
distinguishes an unrequested permission from a denied or revoked permission
after restart. Native scale changes are re-read when the app returns from system
settings. A scale change that does not cause a WebView event while the app stays
active can remain stale until the next authoritative resnapshot.
