# iOS App Access Lock - Delta Spec

## ADDED Requirements

### Requirement: App Lock policy is explicit and persistent

The iOS application supports `EveryLaunch`, `OncePerBoot`, and `Off`. The default
policy and the fallback for an unrecognized persisted value are `EveryLaunch`.

#### Scenario: Persisted policy is absent
Given the application has no persisted App Lock policy
When it resolves the active policy
Then it selects `EveryLaunch`

#### Scenario: Persisted policy is invalid
Given the application has an unrecognized persisted App Lock policy
When it resolves the active policy
Then it selects `EveryLaunch`
And it does not weaken the lock to `Off`

#### Scenario: Operator changes the policy
Given the App Lock settings control is available
When the operator selects a valid policy
Then the application persists that exact policy
And the next policy decision uses the persisted value

### Requirement: Initial setup is exempt until successful completion

App Lock does not request device-owner authentication before initial setup has
produced a usable backend session. Failed or abandoned setup does not record setup
completion.

#### Scenario: Fresh setup begins
Given initial setup is incomplete
When the application starts the setup workflow
Then it issues no App Lock authentication request
And any identity custody authentication remains a separate event

#### Scenario: Setup fails
Given initial setup is incomplete
When backend session preparation fails
Then setup completion remains false
And the failure does not record App Lock satisfaction

#### Scenario: Setup succeeds
Given initial setup is incomplete
When backend session preparation produces a usable session
Then setup completion is persisted
And later launches evaluate the selected App Lock policy

### Requirement: Launch and boot semantics are deterministic

After setup, `EveryLaunch` requires one successful App Lock authentication for
each application process, `OncePerBoot` requires one for each device boot, and
`Off` requires none.

#### Scenario: Every-launch backend retry
Given `EveryLaunch` was satisfied in the current application process
When backend startup retries in that process
Then the retry issues no second App Lock request

#### Scenario: Once-per-boot relaunch
Given `OncePerBoot` was satisfied during the current device boot
When the application cold-launches again during that boot
Then it issues no App Lock request

#### Scenario: Once-per-boot after reboot
Given `OncePerBoot` was satisfied before the device rebooted
When the application launches during the new boot
Then it requires one App Lock authentication

#### Scenario: App Lock is off
Given the persisted policy is `Off`
When the application launches after setup
Then it issues no App Lock authentication request

### Requirement: Authentication gates private session startup

When App Lock authentication is required, private backend startup occurs only
after an `Authenticated` outcome. Every other outcome remains closed and typed.

#### Scenario: Authentication succeeds
Given the active policy requires authentication
When device-owner authentication succeeds
Then the application records policy satisfaction
And it starts the private backend session

#### Scenario: Authentication does not succeed
Given the active policy requires authentication
When device-owner authentication is cancelled, unavailable, or failed
Then the application does not start the private backend session
And it exposes a typed App Lock failure that permits an explicit retry

### Requirement: App Lock and identity custody remain independent

Changing or satisfying App Lock does not create, delete, migrate, unlock, or
alter identity custody. A custody event does not silently satisfy App Lock.

#### Scenario: App Lock succeeds before Keychain access
Given device-owner authentication satisfied App Lock
When the backend later requests Keychain identity access
Then the application treats the Keychain result as separate custody evidence
And App Lock success does not claim identity access succeeded

#### Scenario: App Lock policy changes
Given the application has an existing Keychain-backed identity
When the operator changes App Lock policy
Then identity custody remains unchanged
And no private identity material enters App Lock persistence

### Requirement: Settings presentation is platform bounded

The iOS More screen exposes the current policy and all supported choices with an
accessible label. Other targets do not advertise an operative iOS App Lock.

#### Scenario: iOS settings render
Given the iOS host supplied App Lock policy state
When the More screen renders
Then it presents all three choices with the current choice selected
And it explains that identity custody remains protected separately

#### Scenario: Non-iOS settings render
Given the host is not iOS
When the More screen renders
Then it does not present an operative iOS App Lock control
