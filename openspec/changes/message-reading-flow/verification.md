# Verification — 2026-09-07

- `cargo test -p styrene-dx`: 166 passed, 4 integration tests intentionally ignored.
- `cargo clippy -p styrene-dx --all-targets -- -D warnings`: passed.
- `node --test apps/desktop/tests/thread_viewport.cjs`: 6 passed.
- `swift apps/desktop/tests/viewport_webkit.swift`: passed in native WebKit.
  Exercises initial load, incoming preservation, prepended history, outgoing
  anchoring, late content growth, jump-to-latest, and timestamp formatting using
  the production controller and CSS. This fixture sends no network messages.
- `cargo test -p styrene-ui-app --test theme`: 3 passed after the accompanying
  operator-supplied dark primary color change.
- Final `just mac-app`: `macos-app-ml04jy1a`, passed.
- Final `just ios-build`: `apple-mobile-5j_lzrfl`, passed for the shared theme.
  The previous package build compiled successfully but the lab rejected its
  source-state evidence because Git staging changed during the run. The final
  run retained a stable source state.

## Runtime

The final desktop process is PID 57788, started at 09:31:42 EDT from the canonical
UI clone's `target/dx/styrene-dx/debug/macos/StyreneDx.app`. Live daemon identity
and state were preserved. Evidence in the outer `lab/runs/message-reading-flow/`
includes source fingerprints, initial/final binary hashes, process records,
tests, build logs, and screenshot hashes.

The final capture is window 3367, Messages / RNS Echo Bot. It shows the newest
messages near the composer, content separated from captions, relative times
under an hour, local date/time for older messages, and the softer selected green.
The operator selected the conversation; Accessibility automation is unavailable.
No automated interaction with the live details dialog or live message send is
claimed. Scroll behavior was exercised in the isolated native WebKit fixture.

The desktop owns this message composition. Mobile message layout is unchanged;
attachment preview/download behavior remains a future typed-content integration.
Android/Linux and physical Apple runtime validation were not performed.
