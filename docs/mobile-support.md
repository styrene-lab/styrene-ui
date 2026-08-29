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
| Android permissions and USB | Runtime permission APIs, exact-device `UsbManager` requests, and a bounded Rust JNI byte worker | Bluetooth scan/connect, camera, and notification gates are queried and requested. USB requires an explicit attachment choice after Bluetooth fallback is accepted. Serial opening currently covers CDC ACM and Silicon Labs CP210x devices only |
| iOS permissions | AVFoundation and CoreBluetooth authorization APIs | Camera and Bluetooth status is queried and requested. USB remains unavailable because it has no generic iOS authorization flow |
| iOS notifications | `UNUserNotificationCenter` settings and request callbacks | Snapshots and post-request results use the exact current authorization setting |

Native Android calls use Wry's bounded main-thread pipe and a one-slot result
channel. Dispatch fails closed when no activity or queue capacity is available.
Permission completion is observed after the system dialog returns focus. A
bounded timeout is reported rather than fabricating a decision. Native facts are
re-read for authoritative resnapshots, so stale WebView generations cannot
replace the current platform state.

Android stores a request marker before it opens a permission prompt. This marker
distinguishes an unrequested permission from a denied or revoked permission
after restart. Android configuration callbacks and iOS Dynamic Type notifications
request an immediate authoritative resnapshot after native text-scale changes.

iOS Objective-C calls that cannot satisfy the application workspace's
`unsafe_code = "forbid"` policy are isolated in `styrene-ui-apple-bridge`. Its
safe API exposes only plain authorization values and bounded request tokens;
native objects and pointers do not cross into the application crate.

Android USB enumeration identifies only the current attachment by device ID,
vendor ID, product ID, and device path. Permission callbacks and the current
device list must match that complete identity. Authorization alone does not
imply an open serial link or a connected RNode. After authorization, a dedicated
bounded Rust worker opens a CDC ACM or CP210x byte stream, while the backend owns
KISS framing, RNode detection, radio configuration readback, packet admission,
and bearer truth. Detach closes the attempt and changes the backend USB bearer;
only exact successful configuration readback changes it to connected. Physical
USB behavior and additional USB-to-serial chipsets remain unverified.
