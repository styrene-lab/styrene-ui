# Verification — 2026-09-06

Source: UI base 2875c5f plus this change, backend pin
cdeda5c80d7cacf1811d325239e97d4c980ba773. Unrelated mobile work remains in the
checkout and is outside this desktop change.

- Desktop tests: 152 passed, 4 ignored. Includes neutral discovery/search, deadline
  dropping its pending future, bounded recent outcomes and session isolation.
- Explicit desktop flow corpus: 2 runner tests passed, covering 8 existing journeys.
- Desktop rustfmt and warning-denied Clippy passed. Vendored Wry retains 17 warnings.
- OpenSpec validation passed.
- macOS build: comms-lab/lab/runs/macos-app-4mhrxhbw/result.json.
- Executable SHA256: 0c7e4e37f0005eb9bbc4464570af9af0a10e68995be7ddf231068feec292a520.
- Replaced PID 32453 with PID 48957 from the canonical debug StyreneDx.app bundle;
  launched Live against the lab daemon socket with STYRENE_DX_ROUTE=browse.
- Capture: comms-lab/lab/runs/browse-discovery-feedback/styrene-browse-live.png.
  The capture shows 245 matching hosts, neutral route labels, relative announce
  times, search, separate local pages, and the Browse route selected.

Native click/keyboard checks for rapid replacement, Stop waiting, retry and search
remain unexecuted because Accessibility automation is unavailable. The screenshot
verifies presentation, not those interactions. The daemon's final-stage latency and
remote link establishment are not fixed or validated by this UI change. Linux,
Android and Apple mobile builds were not run. Keep this change unarchived until
native interaction acceptance is recorded.

## NomadNet extraction follow-up — 2026-09-06

Backend dependencies now pin `53e601003f2ced023301ccbbd320af022feb1e74`,
including the extracted `styrene-nomadnet` coordinator. Mobile target-specific
pins match the workspace pin to avoid mixing backend contracts.

The local observation budget is 45 seconds: the default daemon operation has
30 seconds and IPC has a 15-second cleanup allowance. The previous 15-second UI
cutoff could discard an eventual success or the daemon's structured failure.
A paused-clock regression preserves a result arriving at 25 seconds. Explicit
Stop/replacement and pending-read drop behavior are retained.

Validation: 153 desktop tests passed, 4 ignored; warning-denied desktop Clippy
and formatting passed. This does not prove live remote responsiveness. Apple
mobile, Android, and Linux runtime validation remain unexecuted for this pair.
The unpublished backend commit was supplied to the local Cargo Git cache from
the canonical development clone; remote handoff must transfer that exact commit.
