# Verification — 2026-09-07

Root cause: desktop had no mark-read command or call site. Conversation counts
were loaded from the daemon but not refreshed on message/operation events.

- Desktop tests: 167 passed, 4 integration cases intentionally ignored.
- Node viewport tests: 7 passed, including read visibility/focus/history policy.
- Native WebKit viewport regression fixture: passed.
- Warning-denied desktop Clippy: passed.
- Native package: `macos-app-8o5ull9q`, passed.
- Logs, source fingerprint, artifact hash and process evidence are retained in
  outer `lab/runs/unread-tracking/`.

Read-only inspection before launch found seven persisted incoming unread
messages. The UI was rebuilt and reopened in Messages with the existing Live
identity and daemon. Marking these through the actual conversation requires
operator selection; UI automation permission is unavailable. No manual database
change or direct mark-read command was used to manufacture a cleared badge.
The local read command does not send a remote read receipt.

This change is desktop-only. Mobile and backend implementation are unchanged.
