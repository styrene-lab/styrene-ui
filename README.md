# Styrene UI

Shared Rust and Dioxus applications for the Styrene mesh communications system.

The workspace uses Rust 2024, Cargo resolver 3, and Rust 1.97 to match the
authoritative `styrene-rs` workspace contract.

The repository was established from the history-preserving extraction of
`crates/apps/styrene-dx` in `styrene-lab/styrene-rs`. `apps/desktop` is the
authority for the desktop operator console and resolves its backend
contracts from the immutable `styrene-rs` revision recorded in
`docs/extraction-provenance.md`.

`apps/mobile` owns the shared Rust/Dioxus application and one embedded session
on a bounded worker. `apps/mobile-android` and `apps/mobile-ios` are independently
versioned package hosts. Android uses an Android Keystore-wrapped root secret;
Apple devices use Keychain-backed identity storage. Both consume the same
immutable `styrene-rs` revision and shared product state.

The supported platform and capability matrix is in `docs/mobile-support.md`.
Parity-corpus authority and admission status are in `docs/parity-corpus.md`.

## Validation

```bash
cargo fmt --all -- --check
cargo test --workspace --exclude styrene-dx
cargo clippy --workspace --all-targets --exclude styrene-dx --no-deps -- -D warnings
cargo test -p styrene-dx
cargo clippy -p styrene-dx --all-targets --no-deps -- -D warnings -A clippy::large-enum-variant
dx build --package styrene-mobile-android --platform android
dx build --package styrene-mobile-ios --platform ios
```
