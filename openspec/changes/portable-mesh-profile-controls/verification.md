# Planning and preliminary correction evidence

The profile integration is planned, not implemented. OpenSpec validation passed.

A separate Connection dismissal correction replaces the plain details disclosure
with controlled open state, a toggle button, outside-click dismissal, Escape,
Close, and closure after valid activation. Invalid activation retains the panel.

Mac checks passed: desktop cargo check, 143 desktop tests (4 ignored), rustfmt,
and desktop Clippy with -D warnings. The pinned wry dependency emitted existing
warnings separately. The application bundle built through `just mac-app` and was
reopened on the same lab daemon socket. Existing unrelated mobile work remains
in the build source and is not included in this change's commit.

Runtime click/Escape acceptance remains unverified: System Events denied
Accessibility access to osascript. Screen capture permission does not grant it.
Do not claim automated GUI interaction passed. Local evidence is in the
coordination checkout under lab/runs/mesh-profile/ui-*.log, ui-artifact.sha256,
and ui-process.txt; package build evidence is lab/runs/macos-app-fbx78jmb/.
