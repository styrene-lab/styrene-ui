# UI and contract integration

## Header and interaction

Desktop header order: Styrene Mesh, existing identity/build context, flexible
space, Profile: Lab ▾, connection status, Activity. Keep the build visible and
avoid placing private paths or key material in the header. Narrow/mobile layouts
retain the active profile name and status with a sheet opened by the same control.

The profile panel contains the active name, typed identity summary, custody,
save status, saved-profile inventory, and actions: New, Edit, Import, Export.
Selecting an inventory row opens a preview with a separate Activate action.
Advanced connection settings hold Live/Embedded/Fixture and socket configuration.
These are execution modes, not named identities or portable profiles.

Use a panel for selection and a dedicated editor/sheet for forms. Edit includes
name, interfaces/hubs, propagation preferences, and app preferences supported by
the backend. Interface edits use the existing interface service. Configuration
changes must not be described as an identity rotation. Export offers
Configuration only and Profile with identity, with history inclusion only when
supported. Show exactly what is included before the native save/share action.

A clean panel closes by toggle, outside click, Escape, Close, or accepted action.
Return focus to its trigger after keyboard dismissal. Editor dismissal checks
unsaved changes. Long operations persist independently of the panel: closing it
must not imply cancellation. Reopening shows current operation progress.

## Existing implementation and dependencies

Backend 579ee533 already contains operator_profile, ProfileManager,
DaemonProfiles IPC, snapshots/adoption, raw-RNS recovery slots, and Styrene root
backup/recovery. Desktop daemon_bridge already opens managed Local/Quick sessions
and presents backend ProfileInfo on System. Extend these integrations.

The Mac lab launcher currently starts an unmanaged explicit-path daemon. Its
profile name is not yet authoritative ProfileInfo. Adoption requires stopping
that owned daemon and retaining its existing RNS keys. Live clients attached to
external daemons must connect to another endpoint or request supported owner-side
activation, never terminate the external daemon themselves.

Backend prerequisites belong in styrene-rs, with a corresponding backend change
proposal before implementation: typed identity references, legacy adoption,
versioned authenticated encrypted package, bounded staging/validation, secret-free
preview DTOs, capability discovery, and transactional activation outcomes.
Profile export currently copies a snapshot directory; do not present it as an
encrypted archive. Portable media mode requires a media inspector and is distinct
from importing a package through a mobile document picker.

Root recovery gives access to all root-derived key families. Legacy RNS transfer
preserves only that existing RNS key pair. There is no implemented per-device RNS
derivation/delegation contract. Do not offer 'Add another device' as though one
exists. Profile names must never become derivation labels for existing identities.

## Ownership and state

styrene-ui-state owns profile selection, editor drafts, save/conflict state,
operation progress, and generation filtering. styrene-ui-app owns reusable panel,
editor, preview, and identity-custody presentation. styrene-ui-platform owns typed
file/share/custody interaction requests; adapters implement native dialogs.
The daemon owns package validation, authorization, storage, custody and lifecycle.
UI adapters receive opaque protected artifacts or handles, never plaintext keys.

Use existing profile IDs and backend revisions/generations as authoritative.
An unsupported operation is disabled with its specific capability reason. Failed
imports leave active state unchanged. Failed activation reports the actual state;
automatic rollback must not accidentally run both identities.

## Delivery sequence

1. Ship the small Connection dismissal correction independently.
2. Add shared profile summary/inventory presentation and capability-aware controls.
3. Integrate legacy adoption, editing, and owned-runtime activation with backend work.
4. Integrate encrypted export/import, previews, and native file/share adapters.
5. Verify persistence, identity continuity, failure recovery and cross-platform transfer.

No new profile controls are considered delivered merely because this plan exists.
The dismissal correction does not rename Connection until profile data is available.

## Substrate terminology

System's preliminary desktop correction separates RNS/LXMF addressing from public
metadata and labels the app connection Session ready. Shared Profile components
must retain that distinction. IdentityInfo does not provide a canonical Styrene
Identity ID or general signed node binding; absence must remain explicit.
A legacy announce-name role is advertised behavior, not verified Fleet authority.
The coordination evidence is docs/rns-and-styrene-boundaries.md in comms-lab.
