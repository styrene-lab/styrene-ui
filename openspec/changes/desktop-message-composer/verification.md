# Verification

Desktop package tests: 165 passed, four existing tests ignored. Formatting and
warning-denied Clippy passed; vendored Wry retains its existing warnings.

Final source revision `f2eafd6`; `just mac-app` passed with unchanged sources in
`lab/runs/macos-app-0jjbm2qm/result.json` in the coordination checkout. Replaced
the previous desktop process and opened Network in Live mode, ready to select a
peer and Message. Runtime process and binary hash are recorded in
`lab/runs/desktop-message-composer/runtime.json`.

Composer typing, options, native picker and send shortcuts remain manually
unverified because automated clicks are unavailable under the current macOS
Accessibility restriction. No message was sent during validation. Mobile source
and runtime evidence are outside this desktop presentation change.

## Lifecycle controls

`29a44d5`: 166 desktop tests passed, four ignored. Clippy and formatting passed.
The new test covers outgoing/incoming direction, terminal delivery, authoritative
retry eligibility and in-flight cancellation presentation. Mac build passed with
unchanged sources (`lab/runs/macos-app-23t54236/result.json`). Replaced the app
and opened Messages in Live mode; provenance is in the coordination checkout at
`lab/runs/message-lifecycle-controls/runtime.json`. Click testing remains unverified.
