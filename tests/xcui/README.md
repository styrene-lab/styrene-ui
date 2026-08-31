# iOS Packaged UI Tests

Build the Dioxus iOS application with the `styrene-mobile-ios/ui-test` feature.
The feature reads `STYRENE_UI_FIXTURE_ID` at launch and uses the existing
backend-owned fixture corpus instead of starting a live mobile session.

The XCUITest project is independent of Dioxus-generated Xcode output. The test
launches the preinstalled `io.styrene.mesh` application and verifies packaged
WebView navigation. It does not provide VoiceOver or spoken-output evidence.

Run the test with Dioxus CLI and Xcode installed:

```sh
./tests/xcui/test-ios-ui.sh
```

The runner builds the app for `aarch64-apple-ios-sim`, corrects the known
Dioxus `0.8.0-alpha.1` generated platform metadata, and ad hoc signs the ignored
bundle before installation. Set `STYRENE_IOS_APP_PATH` to use an existing
simulator bundle. Set `STYRENE_XCUI_SIMULATOR` and `STYRENE_XCUI_DESTINATION` to
override the default iPhone 17 Pro simulator. Derived data and the result bundle
remain under the ignored workspace `target/` directory.

## Skywave Physical Capture

`testSkywaveParitySmokeCapture` is an opt-in, non-mutating smoke capture for the
installed `co.horsfalldesign.skywave` beta. It retains a screenshot and semantic
snapshot, then verifies background and foreground recovery. It does not inspect
private storage, tap application controls, or establish protocol behavior.

Run the read-only suite on the single paired physical iPhone:

```sh
./test-skywave-ios.sh
```

The script reuses the existing physical XCUITest runner profile, discovers the
paired device at runtime, and keeps all local signing values and device
identifiers in ignored output. It runs launch recovery, top-level inventory,
Identity, Interfaces, Mail Sync, and new-message entry captures.

Review the result attachments for identity, destination, message, and network
data before publishing. An accessibility snapshot is not VoiceOver evidence.
