# Mobile Panel Pass

## Intent

Finish the field-kit treatment on the panels the screen pass left as prose:
the propagation panel, the Bluetooth RNode card, the identity recovery forms,
and the New Message panel, and make every board scale with Dynamic Type.

## Scope

This change covers the propagation panel markup and the stylesheet rules for
the boards in `styrene-ui-app`. It keeps every element identifier and status
attribute that the shell tests and the packaged XCUI suite rely on.

It excludes daemon and IPC behaviour, the desktop application, and Android
work owned by the Nucleus host.

## Success criteria

- The propagation panel opens with a readiness chip, puts node selection and synchronization on one line, and keeps its policy and evidence sentences behind a disclosure.
- Board text uses system text styles so the largest accessibility size scales rows and chips together, and no chip overflows its card.
- The shell tests, workspace validation, and the XCUI suite in both appearances pass.
