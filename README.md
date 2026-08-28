# Styrene UI

Shared Rust and Dioxus applications for the Styrene mesh communications system.

The repository is being established from the history-preserving extraction of
`crates/apps/styrene-dx` in `styrene-lab/styrene-rs`. The active workspace begins
with renderer-neutral presentation state. Extracted desktop source is retained
under `apps/desktop` while its dependencies are converted to standalone,
immutable `styrene-rs` references.

## Validation

```bash
cargo fmt --all -- --check
cargo test -p styrene-ui-state
cargo clippy -p styrene-ui-state --all-targets --no-deps -- -D warnings
```
