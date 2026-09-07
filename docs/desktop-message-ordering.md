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
The desktop suite was rerun against the final `a04d2b57` backend pin: 168 passed,
0 failed, 4 pre-existing ignored tests.

The expanded suites also passed:

- Unix-socket integration: 47 passed, 0 failed, 0 ignored. These assertions cover
  the ordering field in query responses and pushed message events.
- Message storage: 84 passed, 0 failed, 1 initially ignored. The ignored
  `new_database_file_is_private_under_permissive_child_umask` test was then run
  separately with `--exact ... --ignored` and passed, including its child
  process. Total: 85 storage tests passed.

The initial executables stalled at the macOS loader entry point when launched
from the backend's build-cache directory. Copying those same executables to
`/tmp/styrene-order-validation/` allowed both suites to execute successfully.
SHA-256 equality was verified before execution; the test code was not changed,
re-signed, or bypassed. This establishes a successful isolated execution path,
not the underlying macOS cause. Tests used their normal crate working
directories. The storage run took 4.73 seconds and the socket run 0.22 seconds.

Executable SHA-256:

- Socket: `e8065d1b4a071a6ac9afae0b8b9026d60120a04929c87fdb46bdc6d3cd5f7b9d`
- Storage: `f034a62e30e2260deab96b1e0017148a78f805ae56ba4a58e013156269b4a940`

Full logs, exact commands, hashes, and source identity are retained in the
coordination checkout at `lab/runs/message-ordering/completed-validation/`.
Linux/Android and Apple mobile validation remain unexecuted for the additive
dependency update.

Final Mac builds passed: `macos-binaries-6_9fg5fp` and
`macos-app-w8fg6lze`. Both use the canonical output paths. The local daemon and
desktop were restarted. A read-only query over the live Unix socket confirmed
that the original equal-timestamp exchange exposes outgoing sequence 17 and
incoming sequence 18. The same probe failed on the old daemon because the field
was absent. No historical messages were rewritten or new messages sent.
Evidence is in the coordination checkout's `lab/runs/message-ordering/`.
