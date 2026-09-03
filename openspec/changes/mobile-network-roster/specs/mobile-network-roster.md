# Mobile Network Roster - Delta Spec

## ADDED Requirements

### Requirement: Roster rows are fields

Each roster row must place glyph, name, hash, aspect, age, and announce count
in fixed positions, with only the name truncating.

#### Scenario: Long name
Given a peer with a display name longer than the row
When the roster renders
Then the name truncates with an ellipsis
And the age and announce count stay in their column

### Requirement: One place per bearer

Each bearer must appear once on the Network screen, with its configuration
beneath its status, and the TCP row must show the active endpoint while
connected.

#### Scenario: Connected TCP bearer
Given a connected TCP bearer with a configured endpoint
When the Network screen renders
Then the TCP row shows an active chip beside the configured endpoint
And the endpoint input and Apply sit beneath it
