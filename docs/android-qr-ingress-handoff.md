# Android QR Ingress Handoff

This procedure starts QR ingress implementation and E87 verification on
`nucleus`. It does not authorize Apple build or device work on `nucleus`.

## Upstream Contract

- UI branch: `feat/complete-mobile-product-workflows`
- Backend branch: `feat/complete-mobile-product-workflows`
- Backend revision pinned by the UI: `e70c3d6cb140cf5427fc912b32acc318981eaee8`
- OpenSpec: `styrene-rs/openspec/changes/complete-mobile-product-workflows/`
- TDD corpus: `tests/fixtures/mobile-qr-ingress-v1/corpus.json`
- Selected design: system image capture plus bounded Rust `quircs` decoding

Do not add maintained Kotlin, Java, Gradle, or generated Android product source.
Do not move destination validation into the decoder or platform adapter.

## Synchronize Nucleus

Run these commands in the independent UI clone on `nucleus`:

```sh
git fetch origin
git switch feat/complete-mobile-product-workflows
git pull --ff-only origin feat/complete-mobile-product-workflows
git status --short
cargo test -p styrene-ui-platform --test qr_ingress_corpus
```

Stop if the worktree contains conflicting changes. Do not reset unrelated work.

## TDD Sequence

1. Add the failing decoder tests named in the corpus `first_test` fields.
2. Generate QR matrices and JPEG or PNG images in memory. Do not commit image frames.
3. Add these exact dependencies to the owning platform crate:

```toml
image = { version = "=0.25.10", default-features = false, features = ["jpeg", "png"] }
quircs = "=0.10.3"
```

4. Enforce encoded-byte bounds before image parsing.
5. Read dimensions before allocating the decoded frame.
6. Enforce width, height, and pixel bounds from the corpus.
7. Convert the bounded image to one grayscale frame.
8. Reject zero symbols as `no_code` and multiple symbols as `ambiguous`.
9. Pass the decoded bytes through `CandidatePayload` without destination validation.
10. Return `TextAcquisitionCompletion` with the originating generation.
11. Keep errors and `Debug` output payload-free.
12. Compose a single-shot file input with `accept="image/jpeg,image/png"` and `capture="environment"`.
13. Preserve the current destination on denial, cancellation, stale completion, and decode failure.

The capture control may offer the gallery when the Android WebView does not open
the camera directly. This is an acceptable P0 fallback. Record the observed
behavior instead of claiming a continuous scanner.

## Local Gates

Run these gates before device installation:

```sh
cargo test -p styrene-ui-platform --test qr_ingress_corpus
cargo test -p styrene-ui-platform
cargo test -p styrene-ui-app --test mobile_shell
cargo test -p styrene-mobile
cargo clippy -p styrene-ui-platform -p styrene-ui-app -p styrene-mobile --all-targets -- -D warnings
cargo fmt --all -- --check
git diff --check
```

## E87 Preparation

Connect the E87 tablet to `nucleus`, unlock it, and authorize USB debugging.

```sh
adb devices -l
dx build --package styrene-mobile-android --platform android
```

Use the package path reported by `dx`. Install that exact artifact with `adb
install -r`. Record its SHA-256 before installation.

## E87 Evidence Matrix

Run each case from a clean New Message surface:

| Case | Required observation |
|---|---|
| Camera granted | User gesture opens camera or image selection; no earlier permission request occurs |
| Camera denied | Typed denial appears; manual entry and Paste remain usable |
| Capture cancelled | Existing destination remains unchanged; cancellation is not reported as decoder failure |
| Gallery JPEG | One generated canonical QR becomes a candidate and still requires backend validation |
| Gallery PNG | Same result as JPEG |
| No code | Typed `no_code`; no conversation appears |
| Two codes | Typed `ambiguous`; neither value is selected |
| Malformed image | Typed `malformed`; no payload or frame appears in logs |
| Rotation | Active request either survives with its generation or is cancelled without stale mutation |
| Process interruption | Relaunch shows no optimistic contact or conversation from the interrupted scan |
| Stale completion | Old completion cannot replace a newer candidate or failure |
| Invalid decoded text | Backend rejects it and creates no conversation |

For every run, record the UI and backend revisions, APK SHA-256, Android version,
WebView version, corpus case ID, permission state, result code, and outcome. Keep
captured frames and decoded payload text out of retained logs.

## Handoff Back

Commit implementation and deterministic tests separately from physical-device
evidence. Push the same feature branch. Update OpenSpec task `9.5` only after the
complete E87 matrix has retained evidence. Android evidence does not complete
task `9.6` or any Apple acceptance task.
