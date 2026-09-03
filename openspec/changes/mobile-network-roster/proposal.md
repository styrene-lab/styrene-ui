# Mobile Network Roster

## Intent

Give the People roster fixed fields instead of a sentence that wraps, and give
the Network screen one place per bearer where its state and its configuration
live together.

## Scope

The People row markup and grid, the Network bearer board, the endpoint editor,
and the Bluetooth RNode controls in `styrene-ui-app`, with the review capture
that shows a populated roster. Every element identifier and status attribute
stays.

## Success criteria

- A roster row shows glyph, name, hash, aspect tag, age, and announce count in fixed positions; only the name truncates.
- The TCP bearer row carries the endpoint editor and shows the active endpoint when connected; the Bluetooth RNode row carries the adapter controls; no bearer appears twice.
- The review suite captures a populated roster, and the shell tests, validation set, and XCUI suite pass in both appearances.
