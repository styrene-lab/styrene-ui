# Persistent unread tracking

## Intent
Connect desktop viewing to the existing daemon read-state command and keep conversation counts current.

## Scope
Focused/latest-view observation, typed mark-read requests, acknowledged unread projections, and message-event refresh. No remote read receipt or backend protocol change.

## Success criteria
Unread counts come from persistent daemon state; inactive windows and history browsing do not auto-mark.
