# Mobile Screen Pass - Delta Spec

## ADDED Requirements

### Requirement: App Lock presents a lock screen

While the App Lock gate is closed, the shell must render only a lock screen
with one Unlock action, and the action must be disabled while the system
authentication sheet is presented or when no retry handler is attached.

#### Scenario: Cancelled unlock
Given the session failed with an App Lock code and a retry handler is attached
When the shell renders
Then only the lock screen is visible
And its Unlock action is enabled and described by the failure element

#### Scenario: Authentication in progress
Given the session is authenticating
When the shell renders
Then the lock screen states that it is waiting for device authentication
And its Unlock action is disabled

### Requirement: The roster filters and sorts

The People roster must offer a filter over display name, hash, and aspect,
must sort by observation age with the newest first, and must state aspect,
age, and announce count on one line per row.

#### Scenario: Filtered roster
Given four peers of which one has a display name containing "sower"
When the operator types "sower" into the filter
Then one row remains
And the count reads "1 of 4"

### Requirement: Conversation rows preview the latest message

Each conversation row must show the latest message for its peer and the UTC
time of that message when the message carries a timestamp.

#### Scenario: Peer with two messages
Given a peer with two messages of different timestamps
When the conversation list renders
Then the row shows the content of the later message
And the earlier message does not appear in the row
