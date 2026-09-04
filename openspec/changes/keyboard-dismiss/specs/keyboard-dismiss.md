# Keyboard Dismiss - Delta Spec

## ADDED Requirements

### Requirement: A draft can be left without sending

A tap outside any form control must blur the focused field, and the composer
must offer a Done control while the keyboard is up, so the keyboard and the
hidden tab bar never trap the operator in a draft.

#### Scenario: Tap outside the draft
Given a focused message field with the keyboard up
When the operator taps the message history
Then the field blurs and the tab bar returns

#### Scenario: Done
Given a focused message field with the keyboard up
When the operator taps Done
Then the field blurs and the draft is kept
