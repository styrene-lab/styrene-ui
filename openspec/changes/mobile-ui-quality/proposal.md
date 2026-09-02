# Mobile UI Quality

## Intent

Bring the packaged mobile application up to a reviewable visual and
interaction standard, and stop the iOS Bluetooth RNode path from raising an
unexplained system pairing prompt at launch.

## Scope

This change covers the mobile shell in `styrene-ui-app` and its stylesheet, the
iOS BLE control loop in `apps/mobile`, and the screenshot evidence that gates
them. It defines one title per screen, a compact status strip, a semantic tone
system that is honored by the stylesheet, scoped text wrapping, a mobile type
scale, icon navigation, distinct disabled and destructive treatments, explicit
and cancellable RNode reconnection, and per-tab screenshot capture in both
themes.

It excludes daemon behavior, IPC wire changes, the desktop application, Android
platform work owned by the Nucleus host, and VoiceOver evidence.

## Success criteria

- Each screen states its name once, and the header and status strip together use at most a quarter of the viewport on an iPhone 17 Pro.
- No heading or label breaks inside a word; only identifiers wrap anywhere.
- Positive, caution, negative, and neutral states each have a distinct rendering, and no counter, label, or brand mark shares the positive rendering.
- Disabled primary actions are visibly disabled and are never rendered with the destructive or accent fill.
- Primary navigation uses icons with labels and keeps the iOS minimum control size.
- The application never initiates a Bluetooth connection at launch; reconnection is an explicit, cancellable operator action, and a connect in flight is not cancelled by the application while the system may be showing a pairing request.
- Fixture screenshots of every tab in light and dark themes are retained by the UI test suite before and after the change.
