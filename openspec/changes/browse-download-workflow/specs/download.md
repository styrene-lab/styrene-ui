## ADDED Requirements

### Requirement: Automatic download observation
Active downloads SHALL refresh automatically without a manual Refresh control.
Terminal downloads SHALL stop polling. Observation SHALL remain scoped to the
current download and backend session and SHALL NOT accumulate overlapping polls.

#### Scenario: Transfer completion
Given an active download
When the daemon reports verified completion
Then polling stops and the panel says Ready to save rather than Saved.

### Requirement: Native save interaction
Verified completed files SHALL offer a native Save dialog with a basename
suggestion. Cancelling the dialog SHALL perform no save. Save acknowledgement
SHALL be required before reporting Saved. Progress, save and cancellation errors
SHALL be visible without exposing raw backend error strings.

#### Scenario: Choose a destination
Given a verified completed download
When the operator chooses Save and accepts a native dialog destination
Then the typed save command carries that path and success displays the saved path.

### Requirement: Layered download presentation
The primary panel SHALL present a filename, human-readable state, progress and
relevant actions. Hashes, transfer counters and protocol identifiers SHALL appear
under Transfer details. Unknown progress SHALL be indeterminate.

#### Scenario: Observe an incomplete transfer
Given an active download with no reported total
When the panel renders
Then it shows indeterminate progress and a Cancel action, without invented stages.
