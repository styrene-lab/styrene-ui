# Unreads — Delta Spec

## ADDED Requirements

### Requirement: Viewing acknowledges persistent conversation read state
Automatic read marking requires the active, focused, visible thread to show its latest messages.

#### Scenario: Inactive window
Given an incoming message arrives while the window is unfocused or the operator is reading history
When the viewport updates
Then no automatic mark-read command is sent

#### Scenario: Latest messages viewed
Given the focused conversation is showing its latest incoming message
When the viewport reports it as viewed
Then the UI requests the daemon mark-read operation
And the daemon's acknowledged unread count updates the conversation and navigation summary

### Requirement: Failed and stale read updates cannot erase unreads
Counts remain authoritative and generation scoped.

#### Scenario: Mark-read failure
Given a conversation has seven unread messages
When marking it read fails
Then seven remains visible
And the UI offers retry

#### Scenario: Stale acknowledgement
Given a new session has replaced an older session
When the older session's read acknowledgement arrives
Then current unread counts remain unchanged
