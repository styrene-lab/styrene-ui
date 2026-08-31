# Desktop UI Flow Simulation

`use-cases.json` defines the standard desktop Fixture journeys. The Rust runner
opens the same `BackendSession` boundary as the application, applies daemon
events to `DomainStores`, performs each action, and verifies the state that the
UI consumes.

Run all standard journeys with:

```sh
./tests/desktop/test-desktop-flows.sh
```

The suite covers empty, healthy, degraded, active-propagation, and explicit
failure sessions. It also covers network inventory, direct messaging, and page
browsing.

This is deterministic UI-flow simulation. It opens no external interface and
does not provide native window, pointer, keyboard, WebView, or accessibility
tree evidence. Packaged desktop automation requires a stable application bundle
and platform automation target. Keep that evidence separate when it is added.
