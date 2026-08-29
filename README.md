# Styrene UI

Shared Rust and Dioxus applications for the Styrene mesh communications system.

The repository is being established from the history-preserving extraction of
`crates/apps/styrene-dx` in `styrene-lab/styrene-rs`. The active workspace begins
with renderer-neutral presentation state. Extracted desktop source is retained
under `apps/desktop` while its dependencies are converted to standalone,
immutable `styrene-rs` references.

`apps/mobile` owns the shared Rust/Dioxus application and one embedded session
on a bounded worker. `apps/mobile-android` and `apps/mobile-ios` are independently
versioned package hosts. Android uses an Android Keystore-wrapped root secret;
Apple devices use Keychain-backed identity storage. Both consume the same
immutable `styrene-rs` revision and shared product state.

The supported platform and capability matrix is in `docs/mobile-support.md`.

## Validation

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --no-deps -- -D warnings
dx build --package styrene-mobile-android --platform android
dx build --package styrene-mobile-ios --platform ios
```
