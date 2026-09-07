# Browse discovery feedback - Delta Spec

## ADDED Requirements

### Requirement: Discovery is not availability

The directory SHALL use neutral advertisement and route evidence, searchable by
name/hash, sorted deterministically and initially bounded to 20 entries. Local
pages SHALL be separate from discovered nodes. Last-attempt results SHALL be
session-scoped and bounded; they SHALL NOT establish current reachability.

#### Scenario: Advertised node has never been fetched
Given a page-host announcement with no fetch evidence
When the directory renders
Then it identifies the announcement age and route evidence
And it does not show a green available indicator

### Requirement: Page waits are bounded and replaceable

Browse and navigation reads SHALL NOT block the general UI command loop. A new
selection SHALL replace the old local wait. Stop waiting SHALL finish the local
loading state without claiming backend cancellation. Session changes SHALL discard
old work and stale results. Local waits SHALL end after 45 seconds.

#### Scenario: Unresponsive node
Given a page request that never completes
When its local deadline expires
Then the page shows a timeout and a retry action
And other UI commands remain processable
And no remote cancellation is claimed

#### Scenario: New node replaces old wait
Given node A has a pending read
When the operator selects node B
Then node B becomes the current request without waiting for A
And node A's eventual response cannot replace node B

### Requirement: Page feedback reflects known evidence

Pending feedback SHALL name the destination and local deadline. It SHALL NOT
invent intermediate stages where the daemon only returns final evidence. Reported
failures SHALL use safe stage labels and keep technical observations in expandable
details. Announcements, routing and completed page retrieval remain distinct facts.

#### Scenario: No stage events while waiting
Given the daemon has not returned browse-stage observations
When loading is displayed
Then the UI says it is waiting for the daemon and identifies the absence of live stage detail
And Stop waiting is available
