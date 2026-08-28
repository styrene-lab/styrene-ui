# Styrene UI

This repository owns shared Dioxus presentation, renderer-neutral presentation
state, mobile and desktop packaging, platform-service adapters, and UI tests.

`styrene-rs` remains authoritative for Reticulum, LXMF, runtime, transport,
daemon, persistence, and backend fixture contracts. Do not duplicate protocol or
delivery decisions in this repository.

## Boundaries

- `crates/styrene-ui-state`: framework-independent state, reducers, and selectors.
- `apps/desktop`: extracted desktop Dioxus source; excluded until its old relative
  dependencies are replaced with immutable public contracts.
- `tests/fixtures`: versioned copies of backend-owned fixtures with source revision
  provenance.

Keep product state and workflows in Rust. Platform launcher glue must not own
navigation, product state, protocol state, or workflow decisions.

Run formatting, focused tests, and warning-denied Clippy before committing.
