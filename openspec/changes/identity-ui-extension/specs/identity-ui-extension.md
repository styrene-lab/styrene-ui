# Identity UI extension - Delta Spec

## ADDED Requirements

### Requirement: Optional Identity page has a minimal registration boundary

The host SHALL accept one named Identity management page from an optional compiled
extension. Registration SHALL identify the extension, version, page and host
contract revision. The extension SHALL receive a narrow typed Identity client.

#### Scenario: Compatible extension opens its own page
Given a bundled compatible extension with a mock Identity client
When the operator opens its registered page
Then the extension renders its public Identity summary inside the existing shell
And host navigation and theme remain available

#### Scenario: Core application works without the extension
Given an application built without the Identity UI extension
When the operator starts a mesh session
Then basic profile and identity operating summaries remain available
And no advanced Identity management page is registered

#### Scenario: Invalid registration is rejected
Given a registration with a duplicate extension or page ID or incompatible host contract revision
When the host validates registrations
Then it reports the registration error before activation
And the conflicting registration does not replace another page

### Requirement: Provider evidence remains distinct from UI availability

The page SHALL report provider capabilities and failures without inferring identity
binding, successful authentication, or hardware guarantees. Opening the page SHALL
NOT unlock custody, mutate identity, or execute the CLI.

#### Scenario: Provider is unavailable
Given an enabled extension whose provider reports unavailable
When the operator opens the page
Then the page shows the provider's specific unavailable reason
And it offers an explicit retry without fabricating identity information
And mesh navigation remains usable

#### Scenario: Read request fails
Given a mounted page with an available mock provider
When its read-only refresh returns an error
Then the page displays that structured failure and permits retry
And no identity mutation is issued

### Requirement: Extension lifecycle releases owned UI resources

Enable and disable SHALL be idempotent. Navigation and activation SHALL own
separate resource scopes. Disable SHALL unregister the page and release extension
UI subscriptions. The host SHALL persist only the enable preference for this slice.

#### Scenario: Repeated navigation does not accumulate subscriptions
Given an enabled extension
When the operator repeatedly opens and closes its page
Then view subscriptions return to their pre-open count after each close
And closing does not disable the extension or stop the provider

#### Scenario: Disable an open extension
Given the Identity page is open with only read-only work in progress
When the operator disables the extension
Then the host navigates to an available core page and restores usable focus
And the extension page is removed from navigation
And its subscriptions are disposed and late results are ignored
And identity, credentials and profile storage remain unchanged

#### Scenario: Enable preference survives restart
Given the operator disabled the bundled extension
When the application restarts
Then the extension remains disabled
And no provider unlock or prior request is replayed

#### Scenario: Re-enable creates one fresh scope
Given a disabled bundled extension
When enable is requested twice
Then exactly one page and one activation scope exist
And the provider is queried in a new activation generation

### Requirement: Session changes invalidate old extension results

Extension requests and subscriptions SHALL be associated with their activation and
host session generation. Identity/session changes SHALL invalidate prior view state
and prevent results from crossing into the new session.

#### Scenario: Delayed result belongs to an old session
Given an Identity read is pending for session A
When the host switches to session B before the result arrives
Then the page binds to session B and clears session A view state
And the late session A result does not change session B presentation

### Requirement: UI teardown is distinct from backend operation cancellation

Before exposing mutations, integration SHALL define operation ownership,
observation after teardown, capability revocation, and cancellation semantics.
The first spike SHALL use read-only operations and SHALL NOT invent a durable
operation registry. Disabling UI SHALL NOT imply rollback or identity deletion.

#### Scenario: Mutation remains observable after disable
Given a future supported mutation has been accepted with an operation ID and a host observation route
When the operator disables the extension
Then the host retains access to that operation's status
And it does not report the backend operation cancelled merely because UI was removed

#### Scenario: Mutation cannot survive UI teardown safely
Given a future mutation is pending and no independent observation route exists
When the operator requests extension disable
Then disable is blocked with the pending operation and reason identified
And no backend cancellation or rollback is claimed

### Requirement: Identity dogfooding precedes general plugin mechanisms

The implementation SHALL record first-extension lifecycle evidence before adding
runtime loading or a generalized contribution API. Planned scenarios SHALL remain
unexecuted until tested at their actual boundary.

#### Scenario: First extension review records limits
Given the registration, provider and lifecycle scenarios have been exercised
When the first-extension verification is written
Then it records used contract fields, cleanup outcomes, unresolved ownership and exact revisions
And it distinguishes mock, native runtime and device evidence
And mutation scenarios remain deferred until a provider mutation is integrated
