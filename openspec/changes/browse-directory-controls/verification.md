# Verification — 2026-09-06

Implemented in 0558a1c, 6ccd819 and 6ba4062; backend pin unchanged.

- 156 desktop tests passed, 4 ignored. Coverage includes case-insensitive search,
  evidence precedence, filter composition, missing/future timestamp exclusion,
  absent favorites, newest ordering and ties, persistence round-trip and invalid
  or oversized storage rejection.
- Desktop Clippy passed with warnings denied for the application. Existing Wry
  dependency warnings remain. Formatting and OpenSpec validation passed.
- Native Mac package build `macos-app-14hlq268` passed with stable sources.
- Replaced the previous desktop process and opened the Live Browse route.
  Screenshot confirms compact initial controls, separate favorite buttons,
  corrected empty-reader guidance and newest-first ordering: the Pi's recent
  announcement precedes the older public-host entries.
- I2P/Yggdrasil and cached-content placeholders are deliberately unavailable.
  No provider, cache inventory or network probing was added.

Evidence, hashes and process identity are in the coordination checkout at
`lab/runs/browse-directory-controls/`. Initial expanded controls were captured
before the operator requested consolidation; the final capture shows the compact
collapsed view. Native click sequences and expanded final controls were not
independently automated. Persistence/filter behavior is tested at its Rust
boundary. Linux and mobile runtime validation were not executed. Favorites remain
a desktop-local preference, not part of portable identity/profile export.
