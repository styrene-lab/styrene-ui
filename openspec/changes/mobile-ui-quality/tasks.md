# Mobile UI Quality Tasks

Host: macOS workstation (`host/macos-ios`). Android rendering evidence is
owned by the Nucleus host and is not part of this change.

## 1. Evidence Baseline
<!-- specs: mobile-ui-quality -->

- [x] Repair the fixture UI test build so the `ui-test` feature compiles
- [x] Add the per-tab screenshot capture test and retain the 2026-09-02 baseline captures
- [x] Extend the capture to dark appearance and record light and dark baselines before any visual change (`STYRENE_XCUI_APPEARANCE=light|dark`; baselines under `target/ios-xcuitest/review-{before,light,dark}/`)

## 2. Screen Structure
<!-- specs: mobile-ui-quality -->

- [x] Render one screen title per tab and remove the duplicated section label and card heading
- [x] Replace the per-tab header block with the compact status strip and move the Operational summary into a status sheet (the summary lives on the More tab; a strip-level sheet is not needed)
- [x] Add trailing safe-area padding to the strip and verify against a picture-in-picture overlay (padding tracks `env(safe-area-inset-*)`; a picture-in-picture window floats above every app and cannot be reserved, so the header keeps its controls in the leading two thirds)
- [x] Assert the top-quarter content rule in the capture test (`captureTabs` asserts each tab's first content element sits in the top third of the screen at the default text size)

## 3. Tone System
<!-- specs: mobile-ui-quality -->

- [x] Add explicit positive, caution, negative, and neutral rules and a neutral counter style
- [x] Reserve the accent fill for enabled primary actions and the unread indicator
- [x] Unify the disabled treatment across primary and secondary controls
- [x] Check text and control contrast in both appearances and record the values (see the Contrast table in `design.md`)

## 4. Typography and Layout
<!-- specs: mobile-ui-quality -->

- [x] Scope `overflow-wrap: anywhere` to identifiers and adopt the mobile type scale
- [x] Keep helper text inside card padding beneath the control it describes
- [x] Verify the empty states and the identity card at the reference width and at the largest Dynamic Type size (`testCaptureTabScreensAtLargestTextSize`; section headings wrap instead of overlapping)

## 5. Navigation
<!-- specs: mobile-ui-quality -->

- [x] Add bundled tab icons with labels
- [x] Keep the minimum-size and landscape navigation tests passing

## 6. RNode Reconnection
<!-- specs: mobile-ui-quality -->

- [x] Add and run a failing test that a stored approved RNode produces no connect at startup
- [x] Publish the remembered peripheral with reconnect and cancel actions, and stop the deadline once CoreBluetooth reports the connection
- [ ] Surface a lost bond as a diagnostic and repeat the physical launch capture with the RNode in range and out of range (diagnostic `ios_ble_bond_lost` added; out-of-range launch capture pending, in-range capture needs the RNode)

## 7. Closure
<!-- specs: mobile-ui-quality -->

- [x] Retain after captures for every tab in both appearances and compare against the baseline (`target/ios-xcuitest/review-{light,dark}/`, 8 captures each, 17 XCUI tests passing per appearance)
- [ ] Run the desktop and library validation commands and the packaged UI test suite
