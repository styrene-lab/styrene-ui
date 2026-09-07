## ADDED Requirements

### Requirement: Directory discovery controls
The directory SHALL combine name/address search, collection and announce-age
filters, stable name/newest sorting, result counts and reset. Newest announcement
SHALL be the initial and reset ordering. Unknown/future
announce times SHALL be excluded from bounded age filters. No filter SHALL probe.

#### Scenario: Narrow recent hosts
Given a mix of known, unknown and future announce timestamps
When the operator selects an age window and enters a query
Then only matching hosts with known timestamps inside that window appear.

### Requirement: Favorites survive discovery changes
The directory SHALL persist a bounded local favorite address/name list and show
saved hosts missing from discovery without claiming they are reachable. Persistence
failure SHALL be visible and SHALL NOT falsely report a successful save.

#### Scenario: Revisit a favorite
Given a saved host absent from current announcements
When the Favorites collection is selected after reopening the app
Then the host remains listed with unknown announcement evidence.

### Requirement: Future filters are explicit
I2P, Yggdrasil and cached-content controls SHALL be visibly unavailable with an
explanation. They SHALL NOT create fake live hosts or imply offline access.

#### Scenario: Inspect planned sources
Given the current NomadNet-only provider
When the operator opens directory filters
Then I2P and Yggdrasil show Coming later and cached content shows unavailable.
