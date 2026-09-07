# Desktop message ordering

The desktop orders messages by the daemon's history key: `(timestamp,
ingest_sequence)`, ascending. `ingest_sequence` is the persistent insertion
sequence from the daemon's `message_page_keys` table. The daemon assigns it when
it stores a message and retains it through status updates, duplicate insertion,
and restart. Message hashes identify messages; they do not order them.

This fixes a send and its reply reversing when both have the same integer-second
timestamp. The sequence also keeps their order when events arrive out of order,
older pages load, or the operator reopens the conversation. The existing daemon
pagination contract still uses timestamp as its primary key. This change does
not establish causal order between messages with different sender timestamps or
correct remote clock skew.

`MessageInfo.ingest_sequence` is optional for compatibility with older daemons
and sparse events. The desktop retains stable arrival order when sequence data
is absent, reversing newest-first history pages before merging them. That
fallback cannot reconstruct missing persistent ordering evidence. Do not use a
message hash as a substitute. Sequence values belong to one daemon store;
session generation changes must continue to clear the desktop message state.

No database migration or RNS/LXMF wire change is required. The additional IPC
field is backward compatible with clients that ignore unknown map fields.
Mobile and TUI consumers can adopt the same field independently; this change
updates the desktop consumer only.

Regression coverage:

- Backend storage: same-second outgoing/reply order survives duplicate insert,
  marking read, restart, and one-item history pages.
- Backend facade: page, legacy history, and individual-message queries expose
  the same sequence.
- IPC: absent legacy field plus JSON and MessagePack round trips.
- Desktop: reversed live event arrival, paginated history, delivery updates,
  and thread reload retain send-before-reply order.

## Mac validation, 2026-09-07

Backend revision: `a04d2b572deda3aea4749fc1a9481ecff7397fec`.
The focused storage restart/pagination regression and facade projection test
passed, as did the IPC crate tests, 168 desktop tests (4 ignored), and
warning-denied Clippy for the daemon, IPC types, IPC server, and desktop.
The desktop tests ran against `c8cc8726`, before the additional manual socket
serializer change in `a04d2b57`; the final desktop bundle uses `a04d2b57`.

The expanded storage suite and Unix-socket integration suite did not enter their
test harnesses during this run. Process samples showed the test executables
waiting at the macOS loader entry point. Those processes were terminated; the
socket suite's one retry also stalled before test execution. Do not count these
expanded suites as passing. Linux/Android and Apple mobile validation remain
unexecuted for the additive dependency update.

Final Mac builds passed: `macos-binaries-6_9fg5fp` and
`macos-app-w8fg6lze`. Both use the canonical output paths. The local daemon and
desktop were restarted. A read-only query over the live Unix socket confirmed
that the original equal-timestamp exchange exposes outgoing sequence 17 and
incoming sequence 18. The same probe failed on the old daemon because the field
was absent. No historical messages were rewritten or new messages sent.
Evidence is in the coordination checkout's `lab/runs/message-ordering/`.
