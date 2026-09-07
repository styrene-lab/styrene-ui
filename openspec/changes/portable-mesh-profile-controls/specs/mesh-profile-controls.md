# Mesh profile controls — Delta Spec

## ADDED Requirements

### Requirement: Active profile is visible in the application header
The shared application SHALL present the active profile name and saved state
alongside the brand/build context, with connection status shown separately.

#### Scenario: Open profile controls
Given a persistent Lab profile is active
When the operator opens Lab profile controls
Then the panel shows its name, typed identity identifiers, custody summary, and saved state
And it offers profile management actions according to backend capability
And selecting a row alone does not activate a different identity

### Requirement: Profile controls can be dismissed
Profile and connection panels SHALL support trigger toggle, outside dismissal,
Escape, and an explicit Close action without discarding an edit silently.

#### Scenario: Dismiss a clean panel
Given the connection panel is open with no pending saved-profile edit
When the operator presses Escape
Then the panel closes
And the active runtime remains unchanged

#### Scenario: Dismiss a dirty editor
Given a saved-profile editor has unsaved changes
When the operator requests dismissal
Then the UI offers Keep editing and Discard changes
And neither choice writes the profile implicitly

### Requirement: Profile editing reflects backend persistence
Profile edits SHALL use backend validation and revision conflict handling.
The UI SHALL distinguish Saved, Saving, Unsaved changes, and Save failed.

#### Scenario: Save fails
Given an operator has edited a profile
When the backend rejects its save
Then the edits remain available with an actionable error
And the UI does not display Saved

### Requirement: Portable operations preserve identity semantics
Portable packages SHALL distinguish configuration-only data, legacy RNS custody,
and root-backed Styrene Identity recovery. UI code SHALL receive no plaintext keys.

#### Scenario: Import a root-backed profile
Given a supported encrypted package is selected on another device
When the backend authenticates and validates it
Then the UI previews its profile and typed public identity identifiers
And it discloses that root recovery restores authority to derive other purpose keys
And no existing identity is replaced or runtime activated before explicit selection

#### Scenario: Import unsupported hardware settings
Given a valid profile contains an unavailable serial interface
When the operator imports it
Then that interface remains configured but inactive with a mapping-required explanation
And the remaining compatible settings are retained

### Requirement: Profile activation isolates runtime generations
Activation SHALL quiesce an owned prior runtime before starting a replacement.
An externally owned daemon SHALL not be stopped by switching a UI session.

#### Scenario: Switch fails
Given a profile is active
When another profile fails activation
Then the UI reports which profile and runtime remain active or stopped
And stale events from the old generation cannot populate the new profile
And no new identity is silently generated

### Requirement: Existing local state can be adopted without identity replacement
The backend SHALL provide non-destructive adoption of legacy profile state.

#### Scenario: Adopt the Mac lab profile
Given the local lab contains a raw RNS identity, saved interfaces, and message history
When the operator adopts it into managed profiles
Then its RNS identity and saved data remain unchanged
And its custody is labeled legacy RNS rather than a recovered Styrene root

### Requirement: Profiles distinguish substrate identities and service evidence
Profile presentation SHALL distinguish Styrene Identity IDs, RNS identities,
LXMF service addresses, public metadata, and app session readiness. It SHALL NOT
infer Fleet authorization from a name, role advertisement, or connected interface.

#### Scenario: Display a legacy RNS profile
Given a profile reports an RNS identity and LXMF address without a Styrene Identity binding
When the operator opens its identity details
Then the identifiers are labeled by their actual namespace
And names and icons are presented as optional metadata
And missing Styrene Identity evidence is not synthesized from the RNS hash
