# Desktop Network Workflows - Delta Spec

## ADDED Requirements

### Requirement: Aggregate availability reflects displayed actions

The desktop reports aggregate network availability from the authorization state
of the actions currently displayed to the operator.

#### Scenario: All displayed actions are unavailable
Given the session negotiated the operate control plane
And every displayed network action is unauthorized
When the desktop renders network availability
Then it reports the network controls as read-only
And it does not report the controls as ready

#### Scenario: Some displayed actions are available
Given at least one displayed network action is authorized
And at least one displayed network action is unauthorized
When the desktop renders network availability
Then it reports the network controls as partially available
And each action retains its own availability state

### Requirement: Denial text is operator-safe

The desktop presents actionable denial text without exposing internal capability
identifiers in the primary workflow.

#### Scenario: Authorization denies an action
Given a network action is denied by the active session
When the desktop explains why the action is unavailable
Then the explanation uses operator-facing language
And the technical capability identifier is absent from the workflow text

### Requirement: Workflow input is isolated

Each network workflow owns its required input and validates that input separately
from session authorization.

#### Scenario: Native request input changes
Given the operator has entered an active link ID for link control
When the operator changes the native request link ID
Then the link-control value remains unchanged
And native request validation uses the native request value

### Requirement: Observation relationships remain authoritative

The desktop displays relationships between operations, requests, and resources
only when authoritative records provide a correlation identifier.

#### Scenario: Resource correlation is absent
Given a resource observation has no authoritative request correlation
When the desktop renders the resource observation
Then it identifies the relationship as not reported
And it does not infer a relationship from timing or display order

### Requirement: Empty states remain compact and distinct

The desktop distinguishes no observations from unavailable observation data
without allowing empty state text to dominate the workflow.

#### Scenario: No observations exist
Given the session returned an authoritative empty observation collection
When the desktop renders the observation area
Then each observation category uses a compact empty state
And the network controls remain the primary content
