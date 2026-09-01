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

## Physical Custody Handoff

The physical custody tests are disabled by default. Run them only on
`Chriss-MacBook-Pro` against a signed, preinstalled live package. Do not use the
fixture feature for these tests.

The host runner must inject `STYRENE_CUSTODY_ACCEPTANCE=1` into the test-runner
environment. A clean-install or clean-data step is external to XCTest and must
occur before
`testPhysicalIdentityCustodySurvivesTerminationAndRestart`. The test retains
screenshots and verifies these public facts:

- requested and active storage are `Apple Keychain`.
- protection is `Platform protected`.
- authentication is `Device authentication`.
- availability is `Available` and downgrade is `None`.
- the 32-hex public destination is unchanged after termination and relaunch.

The assigned host runner owns the destructive reset, baseline installation,
normal restart, explicit forced termination, in-place upgrade, and relaunch
steps. XCTest verifies the rendered custody projection and public identity at
the required checkpoints. It does not perform package installation or device
reset operations.

Use `testPhysicalRestoredIdentityCustody` after an in-place package upgrade.
Inject the retained first-launch destination as `STYRENE_EXPECTED_IDENTITY` in
the test-runner environment. The value is public identity metadata. It is not a
secret or a private key.

The tests do not establish a pass claim by their presence. Retain the signed
application digest, source revisions, host class, OS version, package versions,
test result bundle, and screenshots for each executed handoff. Do not commit
device identifiers, signing identifiers, provisioning data, or local paths.
Android custody execution remains assigned to `nucleus` and requires its own
packaged runner.
