# Mobile Screen Pass

## Intent

Carry the field-kit corpus through every screen of the mobile shell so that an
operator sees status boards and rosters rather than forms and prose, and so
that App Lock presents a lock screen rather than a failure banner behind a
system sheet.

## Scope

This change covers the People roster, the Messages conversation list, the
Network and More screens, and the App Lock presentation in `styrene-ui-app`,
with the stylesheet rules that carry them. It keeps every element identifier,
state attribute, and action that the shell tests and the packaged XCUI suite
rely on.

It excludes daemon and IPC behaviour, the desktop application, Android work
owned by the Nucleus host, and the system authentication sheet itself, which
iOS draws.

## Success criteria

- The People roster is one framed list with a filter, sorted by recency, and every row states aspect, age, and announce count on one technical line.
- Each conversation row shows the latest message preview and its UTC time.
- App Lock renders a dedicated lock screen with one Unlock action while the gate is closed, and nothing else of the shell is visible.
- Network opens on the bearer board; the endpoint and display-name editors put their input and action on one line.
- Permission and custody values carry tone glyphs and render in the technical face.
- The shell tests, workspace validation, and the XCUI suite in both appearances pass.
