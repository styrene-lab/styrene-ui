# Message reading — Delta Spec

## ADDED Requirements

### Requirement: Thread scrolling preserves operator intent
The first loaded thread and new outgoing messages anchor to the latest message. Incoming messages follow only while the operator is already near the bottom.

#### Scenario: Outgoing message while reading history
Given the operator has scrolled up
When a new outgoing message appears
Then the viewport reveals the newest message

#### Scenario: History preservation
Given the operator is reading earlier messages
When incoming messages or earlier history are inserted
Then the visible reading anchor stays in place
And new incoming messages expose a jump-to-latest control

### Requirement: Message content is separate from diagnostics
Message text and attachment metadata appear in the bubble. Timestamps and details controls appear beneath it.

#### Scenario: Default card
Given a message with text and attachment metadata
When the message renders
Then text and attachment metadata are visible
And delivery diagnostics are absent until requested in a dialog

### Requirement: Captions use recency and local date
Messages under one hour use recency; older messages use local calendar date and time.

#### Scenario: Age crosses one hour
Given a message is 59 minutes old
When its age reaches one hour
Then its caption switches from recency to local date and time
