# iOS App Access Lock

## Intent

Define and test the iOS App Lock policy that gates private session startup after
initial setup. The merged implementation needs deterministic policy, persistence,
failure, and reboot coverage before its partial physical observations can support
the complete behavior.

## Scope

This change covers the three App Lock choices and strict persisted-value fallback.
It also covers setup exemption, launch and boot satisfaction, authentication
outcomes, startup ordering, settings presentation, and bounded physical evidence.

This change does not define Keychain accessibility, identity migration, identity
backup protection, or private-key custody. App Lock and identity custody remain
independent security boundaries.

## Success criteria

- Pure tests cover every policy decision without requiring Apple frameworks.
- Private session startup occurs only after required device-owner authentication.
- Setup, launch, and reboot state has explicit persistence and failure behavior.
- The UI exposes the current policy without implying that App Lock proves custody.
- Physical evidence covers reboot, Off, negative outcomes, and duplicate-prompt prevention.
