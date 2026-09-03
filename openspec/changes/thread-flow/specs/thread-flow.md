# Thread Flow - Delta Spec

## ADDED Requirements

### Requirement: The thread reads newest at the bottom

The message history must place the newest message beside the composer and
open scrolled to it, and each card must keep only its body, state, and
failure reason visible with the rest behind one disclosure.

#### Scenario: Two messages
Given two messages with different timestamps
When the thread renders
Then the newer message is nearest the composer
And its delivery method and evidence are behind a "Delivery details" disclosure

### Requirement: In-flight state is bounded

The composer must clear its in-flight state when a new outbound record for
the conversation appears, or after a bounded wait, whichever comes first.

#### Scenario: Send fails
Given a send that fails and is recorded
When the failed record appears
Then the composer reads "Send" again
