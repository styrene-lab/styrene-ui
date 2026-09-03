# Composer Sending State - Delta Spec

## ADDED Requirements

### Requirement: A send in flight is visible

After Send is submitted, the composer must show the attempt as in flight and
must not accept another submit until the backend generation advances.

#### Scenario: Submit
Given an enabled composer with a draft
When Send is submitted
Then the control reads "Sending…" and is disabled
And the status line says the message will appear once the backend accepts it

#### Scenario: Backend advances
Given a send in flight
When the next backend snapshot arrives
Then the control reads "Send" again
