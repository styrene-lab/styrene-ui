# Design

Reuse the session-owned command loop for one-second download queries. Only an
active current download produces a query; commands run serially and missed ticks
are skipped. Generation/download guards reject obsolete queued actions.

Use rfd 0.17.2 with exactly Dioxus's existing xdg-portal feature selection and
native macOS/Windows implementations. The dialog runs asynchronously, reports
cancellation by returning None, and never supplies an automatic save destination.
The remote path contributes only a basename suggestion. The daemon still owns
verified bytes and atomic save; no backend/wire changes are made.

Save/cancel operation state and redacted action errors live in ContentStore and
reset on session replacement. Dialog state belongs to a component keyed by
download ID. Saved is distinct from Completed. Transfer byte counters may include
protocol overhead and are not advertised as the file's size.

The existing daemon saves new files without overwriting existing destinations.
If a chosen destination exists or is unwritable, report a save failure and allow
another selection. A native dialog does not authorize the daemon to violate that
existing contract. Save paths are interpreted by the attached daemon; this change
supports the local desktop deployment, not file export from a remote daemon.
