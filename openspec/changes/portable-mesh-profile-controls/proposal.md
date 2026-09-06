# Portable mesh profiles in the shared application header

## Intent

Operators need to recognize, retain, edit, and move their mesh setup from the
application. The desktop Connection disclosure currently exposes runtime modes
and a socket path, rather than a named persistent profile.

## Scope

Add shared profile presentation and operations, with desktop header and mobile
sheet adapters. Reuse backend managed profiles and identity recovery. Integrate
legacy local state without changing its RNS identity. Define an encrypted package
contract in styrene-rs before exposing portable export/import as available.

Distinct-device derivation/delegation, history merging, and bundling executable
apps across operating systems are outside this change. A profile package moves
between compatible platform-specific apps.

## Success criteria

- The header identifies the active profile, connection status, and saved state.
- Profile actions persist through restart and preserve identity continuity.
- Portable import validates and previews before installation and activation.
- Legacy RNS custody and root-backed Styrene Identity remain distinguishable.
- Unavailable operations have specific explanations; no silent runtime fallback.
- Apple and Android evidence is collected separately at an exact backend/UI pair.
