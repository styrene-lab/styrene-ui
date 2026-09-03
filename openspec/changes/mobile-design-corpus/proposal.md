# Mobile Design Corpus

## Intent

Replace the stock-application feel of the mobile shell with a field-kit
visual language: hard edges, hairlines, machine type for machine data, tone
markers that survive without colour, and texture instead of elevation. The
existing bone, olive, and terracotta palette stays; the softness around it
goes.

## Scope

This change covers the shared mobile stylesheet in `styrene-ui-app`, the
design tokens it exposes, and the component treatments built on them: header,
status strip, chips and counters, cards, buttons, form controls, the message
workspace, the Bluetooth scan indicator, and the destination bar. It defines
the corpus that later screen-level work on People, Messages, Network, and More
builds on.

It excludes markup and behaviour changes in the shell, daemon or IPC work, the
desktop application, Android platform work owned by the Nucleus host, and any
change to the fixture corpus.

## Success criteria

- Every token that the stylesheet contract names still exists, and every radius token resolves to zero.
- No component uses a drop shadow; depth comes from hairlines, corner ticks, and surface steps.
- Every value produced by a machine (hashes, counts, timestamps, phases, state labels) renders in the technical face.
- Positive, caution, negative, and neutral tones each carry a distinct marker glyph as well as a distinct colour.
- Every text-on-background pair in both themes meets a 4.5:1 contrast ratio, and every hairline against its surface meets 3:1.
- The stylesheet contract test, the packaged XCUI suite, and the review captures in light and dark pass on the restyled corpus.
