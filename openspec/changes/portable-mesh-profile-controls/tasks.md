## 1. Header and interaction
<!-- specs: mesh-profile-controls -->
- [ ] Add shared profile state/selectors and header summary with typed identity and save state.
- [ ] Add profile panel/editor, keyboard focus handling, mobile sheet layout and dirty-edit dismissal.
- [ ] Move execution-mode controls into Advanced while retaining explicit Live failures.
- [ ] Test narrow layouts, keyboard operation, outside dismissal and operation continuity.

## 2. Backend integration and persistence
<!-- specs: mesh-profile-controls -->
- [ ] Create the corresponding styrene-rs proposal for identity references, adoption, package and activation contracts.
- [ ] Map existing DaemonProfiles operations through desktop/mobile adapters with capability reasons.
- [ ] Implement legacy lab adoption with unchanged RNS keys, interfaces and history.
- [ ] Connect editor persistence and revision conflicts; retain drafts on failure.
- [ ] Implement activation with owned/external daemon distinctions and generation-filtered events.

## 3. Portable packages
<!-- specs: mesh-profile-controls -->
- [ ] Implement backend authenticated package validation, bounded staging and non-destructive import.
- [ ] Integrate configuration-only and identity-inclusive export with authority/inclusion preview.
- [ ] Add native file/share adapters and restore into supported local custody.
- [ ] Preserve unsupported hardware settings with explicit local mapping requirements.

## 4. Acceptance
<!-- specs: mesh-profile-controls -->
- [ ] Run shared state/component tests, desktop tests, formatting and warning-denied Clippy.
- [ ] Capture macOS runtime evidence for activation, editing, persistence and failure states.
- [ ] Verify identity-preserving encrypted transfer on Apple and Android against an exact revision pair.
- [ ] Verify wrong-password, corruption, conflict, cancellation and interrupted-operation cleanup.
- [ ] Record any unavailable native/hardware lanes; do not infer them from mock adapters.
- [ ] Add shared typed identity/address projections and capability provenance; distinguish session readiness from network delivery and Fleet authorization.
