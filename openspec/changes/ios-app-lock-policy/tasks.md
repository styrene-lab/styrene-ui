# iOS App Access Lock Tasks

## 1. Pure policy decisions
<!-- specs: ios-app-lock -->

- [ ] Add failing table tests for default, valid, and invalid persisted policy values
- [ ] Add failing decision-matrix tests for setup state, all policies, launch identity, boot identity, and prior satisfaction
- [ ] Add failing tests proving backend retry does not duplicate an `EveryLaunch` request and reboot invalidates `OncePerBoot`
- [ ] Extract the minimal pure Rust decision function and keep Apple state acquisition in the platform adapter

## 2. Authentication and startup ordering
<!-- specs: ios-app-lock -->

- [ ] Add a fake authenticator and backend-start spy before changing the startup owner
- [ ] Prove `Authenticated` is the only outcome that starts a required private session
- [ ] Prove setup and legacy custody migration paths issue no App Lock request
- [ ] Record authentication and setup satisfaction only after their corresponding successful events

## 3. Persistence adapters
<!-- specs: ios-app-lock -->

- [ ] Add failing isolated-store tests for policy, setup completion, and launch or boot satisfaction
- [ ] Add failing tests for absent and malformed values plus boot-identity and clock-change behavior
- [ ] Replace direct global decision reads with a bounded adapter around the tested policy inputs

## 4. Presentation
<!-- specs: ios-app-lock -->

- [ ] Add component tests for iOS-only visibility, all choices, current selection, change handling, accessible labeling, and disabled guidance
- [x] Retain the merged assertion that App Lock presentation does not conflate identity custody

## 5. Physical evidence and verification
<!-- specs: ios-app-lock -->

- [x] Retain the signed-build observations for fresh setup, one default cold launch, and one same-boot once-per-boot relaunch from UI PR #12
- [ ] Verify same-process retry, post-reboot once-per-boot, `Off`, cancellation, unavailable authentication, and failed authentication on an iPhone
- [ ] Record App Lock and any independent Keychain prompts as separate observations
- [ ] Run formatting, focused tests, warning-denied iPhoneOS Clippy, OpenSpec validation, and clean package checks
