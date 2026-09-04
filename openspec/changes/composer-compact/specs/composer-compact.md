# Composer Compact - Delta Spec

## ADDED Requirements

### Requirement: The composer is one line plus a method

The composer must present the draft field and an icon Send control on one
line, the delivery method as one small control beneath, and a status
sentence only when the send is blocked or needs explanation.

#### Scenario: Ready
Given an enabled composer with a draft and a ready method
When it renders
Then Send is an icon with the accessible name "Send"
And no status sentence is shown

#### Scenario: Blocked
Given a composer whose selected method is not ready
When it renders
Then the status sentence names the reason
