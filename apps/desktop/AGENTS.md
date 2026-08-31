# styrene-dx

**Status: Desktop operator console and candidate authority. `publish = false`.**

Dioxus desktop operator console for Live, Embedded, and Fixture sessions. It
uses the workspace-pinned Dioxus version and typed daemon and
interoperability-runner contracts without owning protocol behavior.

## Structure

```
src/
  main.rs          - Dioxus app root, profile lifecycle, routes
  backend.rs       - Live, Embedded, and Fixture backend sessions
  daemon_bridge.rs - Typed IPC request broker
  scenario.rs      - Protocol Lab runner boundary and evidence projection
  state.rs         - Presentation domain types
  stores.rs        - Generation-gated domain stores and diagnostics
  components/      - Routed operator pages and controls
  assets/          - Static assets (CSS, images)
```

## Build

```bash
cargo build -p styrene-dx          # desktop
dx serve                           # dev server (requires dioxus-cli)
```

Included in the workspace. CI runs its deterministic Fixture and component
coverage separately because Linux builds require GTK and WebKit system packages.

## Dependencies

- Workspace-pinned `dioxus` with the desktop renderer
- `styrene-ipc` and `styrene-ipc-server` for typed local daemon sessions
- `styrene-interop-runner` for bounded Protocol Lab execution

## Notes

- Runtime profiles are explicit. Live connection failure never starts Embedded mode.
- Fixture mode opens no daemon process or external network interface.
- Live Protocol Lab scenarios run in a separate `styrene-interop` process. Set `STYRENE_DX_LIVE_INTEROP=1`. Install `styrene-interop` beside `styrene-dx`, or set `STYRENE_DX_INTEROP_RUNNER` to its executable path.
- The primary terminal client remains `styrene-tui` (ratatui).
