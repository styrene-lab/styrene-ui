# Desktop workspace verification

Date: 2026-09-06. Host: Chriss-MacBook-Pro (macOS).

## Implemented

Summary rail, separate tools, responsive drawer CSS, actionable Overview,
Messages/Delivery nesting, Browse naming, legacy route hints and app-owned page
continuity. No Identity or backend source changes. Concurrent mobile work preserved.

## Software evidence

- `cargo test --locked -p styrene-dx`: 147 passed, 4 ignored.
- `tests/desktop/test-desktop-flows.sh`: 2 tests passed, exercising 8 corpus journeys.
- `cargo clippy --locked -p styrene-dx --all-targets -- -D warnings`: passed.
  Vendored Wry emits existing dependency warnings; the desktop crate passes.
- `rustfmt --check --edition 2024` on changed desktop Rust files: passed.
- OpenSpec validation: passed. This does not establish runtime acceptance.

Projection tests cover unknown state, registration order, legacy routes, Delivery's
Messages parent, interface errors, discovery versus Fleet management, and bounded
activity. The browse continuity case covers app-owned state, not pointer navigation.

## Artifact and captures

Built with comms-lab `just mac-app` from UI base `a2baf72` plus the recorded desktop
working changes. Backend pin: `cdeda5c80d7cacf1811d325239e97d4c980ba773`.
The build record retains the full dirty-source fingerprint, including concurrent
mobile changes. The documentation commit is subsequent to this build.

Build record: `comms-lab/lab/runs/macos-app-8183_byi/result.json`.
Executable: UI `target/dx/styrene-dx/debug/macos/StyreneDx.app/Contents/MacOS/styrene-dx`.
SHA-256: `aee55ddcf1050ddc0a514dd98b8959b0e4a710f00a70de0640e7f66703fd7e02`.

Captures and hashes are retained in the coordination checkout under
`lab/runs/desktop-workspaces/evidence.json`:

| Surface | Process | Observed result |
|---|---|---|
| Live Overview | 32453 | Three saved interfaces, two connected clients, readable traffic, no discovered-peer Fleet count |
| Fixture active-scenario Delivery | 32212 | Messages selected, Conversations/Delivery navigation, existing propagation queue visible |
| Fixture healthy Browse | 32424 | Browse selected, page hosts and existing address/navigation controls visible |

The first Delivery capture showed another foreground application and was rejected.
The first Browse capture showed Messages and was rejected. Only captures matching
the intended selected route are retained as evidence.

## Remaining acceptance and observations

macOS denied Accessibility access to osascript/cliclick. Pointer/keyboard flows,
focus return, and narrow-window drawer interaction remain unexecuted. Capture
permission does not establish interaction permission. Linux and mobile runtime
lanes were not run; no cross-platform acceptance is claimed.

The existing active-scenario propagation fixture displays historical timestamps
and a waiting standard-runtime snapshot; this shell change does not resolve that
fixture's stale clock or expand its service evidence.

Keep this change unarchived until remaining native interaction acceptance is
recorded. I2P/Yggdrasil handoff, bookmarks, profile CRUD and Identity extension
implementation are outside this rearrangement.
