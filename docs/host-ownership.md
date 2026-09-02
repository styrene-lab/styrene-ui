# Host Ownership After Consolidation

`main` pins the consolidated styrene-rs hardening corpus. The remaining work
is split by the host that can produce its evidence, on long-lived branches cut
from `main` on 2026-09-02. Finished work lands on `main` through pull requests.

| Host | Branch | Owns |
|------|--------|------|
| macOS workstation | `host/macos-ios` | macOS desktop and iOS work |
| Nucleus (Linux) | `host/linux-android` | Linux desktop and Android work |

The full assignment rule and per-host task lists are in
`styrene-rs/docs/host-ownership.md`. This repository follows the same rule.

## macOS workstation

- `openspec/changes/ios-app-lock-policy`: the physical iPhone matrix and the
  separate App Lock versus Keychain prompt observations.
- `openspec/changes/desktop-network-workflow-polish` on macOS: keyboard order,
  labels, disabled guidance, destructive confirmation, fixture captures, and
  the Live-failure and Embedded smoke checks.
- The iOS host, custody, QR, and packaging evidence for the mobile application.

## Nucleus

- The Android host, BLE, packaging, emulator, and physical evidence for the
  mobile application.
- The Linux desktop checks for the desktop workflow polish.

## Working agreement

- Rebase the host branch on `main` before opening a pull request.
- Do not tick another host's task or carry its evidence files.
- The styrene-rs pins move only on `main`.
