# Styrene UI

Shared Rust and Dioxus applications for the Styrene mesh communications system.

The repository is being established from the history-preserving extraction of
`crates/apps/styrene-dx` in `styrene-lab/styrene-rs`. The active workspace begins
with renderer-neutral presentation state. Extracted desktop source is retained
under `apps/desktop` while its dependencies are converted to standalone,
immutable `styrene-rs` references.

`apps/mobile` is the Rust-only Dioxus launcher for iOS and Android. Mobile builds
pin `styrened` and `styrene-ipc` to an immutable `styrene-rs` revision and own
one embedded session on a bounded worker. Apple devices use Keychain-backed
identity storage, and Android uses an Android Keystore-wrapped root secret.

## Validation

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --no-deps -- -D warnings
```
