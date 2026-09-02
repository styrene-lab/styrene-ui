# Mobile UI Quality Tasks

Host: macOS workstation (`host/macos-ios`). Android rendering evidence is
owned by the Nucleus host and is not part of this change.

## 1. Evidence Baseline
<!-- specs: mobile-ui-quality -->

- [x] Repair the fixture UI test build so the `ui-test` feature compiles
- [x] Add the per-tab screenshot capture test and retain the 2026-09-02 baseline captures
- [ ] Extend the capture to dark appearance and record light and dark baselines before any visual change

## 2. Screen Structure
<!-- specs: mobile-ui-quality -->

- [ ] Render one screen title per tab and remove the duplicated section label and card heading
- [ ] Replace the per-tab header block with the compact status strip and move the Operational summary into a status sheet
- [ ] Add trailing safe-area padding to the strip and verify against a picture-in-picture overlay
- [ ] Assert the top-quarter content rule in the capture test

## 3. Tone System
<!-- specs: mobile-ui-quality -->

- [ ] Add explicit positive, caution, negative, and neutral rules and a neutral counter style
- [ ] Reserve the accent fill for enabled primary actions and the unread indicator
- [ ] Unify the disabled treatment across primary and secondary controls
- [ ] Check text and control contrast in both appearances and record the values

## 4. Typography and Layout
<!-- specs: mobile-ui-quality -->

- [ ] Scope `overflow-wrap: anywhere` to identifiers and adopt the mobile type scale
- [ ] Keep helper text inside card padding beneath the control it describes
- [ ] Verify the empty states and the identity card at the reference width and at the largest Dynamic Type size

## 5. Navigation
<!-- specs: mobile-ui-quality -->

- [ ] Add bundled tab icons with labels
- [ ] Keep the minimum-size and landscape navigation tests passing

## 6. RNode Reconnection
<!-- specs: mobile-ui-quality -->

- [x] Add and run a failing test that a stored approved RNode produces no connect at startup
- [x] Publish the remembered peripheral with reconnect and cancel actions, and stop the deadline once CoreBluetooth reports the connection
- [ ] Surface a lost bond as a diagnostic and repeat the physical launch capture with the RNode in range and out of range (diagnostic `ios_ble_bond_lost` added; out-of-range launch capture pending, in-range capture needs the RNode)

## 7. Closure
<!-- specs: mobile-ui-quality -->

- [ ] Retain after captures for every tab in both appearances and compare against the baseline
- [ ] Run the desktop and library validation commands and the packaged UI test suite
