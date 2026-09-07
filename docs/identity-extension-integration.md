# Desktop Identity extension integration

## Source boundary

This adapter starts from UI revision `7b757aa804b23cb579e20d6ac161f84b791529ca`.
It consumes Identity revision `eaa5223ddafa28a5de88caa6e132bf6df1cc3eb1` and the
existing backend pin `a04d2b572deda3aea4749fc1a9481ecff7397fec`.
The manifest and lockfile pin both Identity crates to the same immutable source.

The `styrene-dx` default feature `identity-extension` bundles one read-only shared
page. Build with `--no-default-features` to omit the component. The host retains
its core System identity and profile controls in both builds.

The host owns navigation, the enable preference, and the session context. The
client receives a public snapshot and has no custody or arbitrary IPC capability.
Context changes invalidate page generations. Fixture output is explicitly labeled.
There are no extension mutations or extension-owned pending operations in this slice.

## Operator controls

Use **Identity tools** in the host header to enable or disable the page.
The setting persists as versioned JSON in `identity-extension.json` under the first
available settings root:

1. Absolute `STYRENE_DX_SETTINGS_ROOT`.
2. `$XDG_CONFIG_HOME/styrene-desktop`.
3. `$HOME/.config/styrene-desktop`.

The file contains only the schema version and enable preference. Writes use a
temporary file and atomic replacement. Read failures disable the control and
display a status message. A failed save preserves the current in-memory setting.
Disabling the component while its page is selected returns to Overview.

For a disposable Fixture session, set `STYRENE_DX_PROFILE=fixture`,
`STYRENE_DX_ROUTE=identity`, and an isolated absolute `STYRENE_DX_SETTINGS_ROOT`.

## Backend handoff: canonical overview

Live `IdentityInfo` exposes transport identifiers and custody metadata. It does
not supply the canonical Styrene Identity signing public key. The adapter returns
typed `Unsupported` for connected non-Fixture sessions and `ProviderUnavailable`
for disconnected sessions. It never derives a canonical ID from a transport hash.

The backend owner must expose a versioned, read-only canonical overview associated
with the current session generation before this page can show live Identity data.
The contract must distinguish absence, unsupported behavior, and provider failure.
Opening the page must not unlock custody. Acceptance must cover reconnects, profile
switches, old completions, unavailable providers, and mismatched expected identities.

## Software evidence

Checks ran on macOS, Rust 1.97.0, `aarch64-apple-darwin`, against the source changes
in the commit containing this document. Cargo output used an isolated target directory.

- `cargo check -p styrene-dx`: passed.
- `cargo test --locked -p styrene-dx`: 171 passed, 4 ignored.
- `cargo test --locked -p styrene-dx --no-default-features`: 168 passed, 4 ignored.
- `cargo clippy --locked -p styrene-dx --all-targets -- -D warnings`: passed after
  replacing new and pre-existing test `unwrap()` calls with descriptive `expect()` calls.
  The existing vendored Wry dependency emitted 17 warnings.
- `cargo fmt -p styrene-dx`: completed.

These checks establish software behavior, not native interaction acceptance.
macOS Accessibility access was denied during this session. Native navigation,
restart persistence, focus, and assistive-technology acceptance remain unexecuted.
Linux, Android, iOS, and hardware custody evidence are not established here.
