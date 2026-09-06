# Desktop workspaces - Delta Spec

## ADDED Requirements

### Requirement: Stable navigation presents operator workspaces

The desktop SHALL expose Overview, Messages, Browse, Network, Fleet, Lab and
System in that order, separating Lab/System as tools. Delivery SHALL be a child
of Messages, not a top-level destination. Unknown or unavailable observations
SHALL NOT be rendered as successful zero counts.

#### Scenario: Navigate to delivery
Given the desktop is showing Overview
When the operator selects Messages then Delivery
Then existing propagation diagnostics are available
And Messages remains selected in primary navigation

#### Scenario: Narrow window
Given a narrow desktop window
When the operator opens navigation
Then all destination names and summaries remain accessible
And selecting a destination closes the drawer

### Requirement: Overview prioritizes actionable state

Overview SHALL identify session/profile context, display reported interface
conditions with navigation to Network, and show at most five recent activity
entries. It SHALL distinguish session readiness from network connectivity.

#### Scenario: Failed connection
Given the daemon reports a saved interface with an error
When Overview is displayed
Then that interface and error are visible with a Network action
And the session is not described as a successful connection to that peer

#### Scenario: No status yet
Given no runtime status has been received
When Overview and the navigation rail render
Then their summaries identify loading or unavailable state
And they do not infer healthy empty inventories

### Requirement: Browse preserves native page workflows

Browse SHALL retain the existing NomadNet address, navigation, forms, files and
page-host controls. Navigation away SHALL NOT clear the backend page session,
loaded page or transfer state. Unsupported external browser services SHALL NOT
be advertised as implemented launch actions.

#### Scenario: Resume a page
Given a NomadNet page is loaded
When the operator visits Messages and returns to Browse
Then the same page session and address remain available
And no new browse request is issued solely by selecting Browse

### Requirement: Existing route entry points remain meaningful

Legacy command/content/propagation route hints SHALL resolve to Overview,
Browse and Messages Delivery respectively. Network subroute hints SHALL retain
their existing meaning.

#### Scenario: Legacy delivery entry
Given the launch route hint is propagation
When the desktop starts
Then Messages Delivery is selected
