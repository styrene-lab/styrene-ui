# Design

## Direction

The shell should read as a piece of field equipment, not as a consumer
application. Four rules carry that:

1. **Edges are hard.** Every radius is zero. Cards, controls, chips, and the
   navigation strip are rectangles bounded by one-pixel hairlines. Cards carry
   reticle ticks at their four corners in the strong line colour.
2. **Machines speak monospace.** Anything a machine produced renders in the
   technical face: hashes, counts, ages, phases, state labels, kickers, and
   form labels. Names of things people chose, headings, and body copy stay in
   the condensed interface face.
3. **Tone is a glyph before it is a colour.** Positive, caution, negative, and
   neutral states carry `●`, `▲`, `✕`, and `○` markers respectively, drawn by
   the stylesheet from `data-tone`. Colour reinforces the glyph; it never
   carries the meaning alone.
4. **Texture replaces elevation.** No shadows. The canvas carries a faint
   diagonal hatch, the message history a faint grid, the scan indicator hazard
   stripes. All texture sits below 4 percent contrast so it never competes
   with content.

## Tokens

| Token | Light | Dark | Role |
|---|---|---|---|
| `--canvas` | `#e6e2d3` | `#070907` | page ground |
| `--surface` | `#f1eee3` | `#0d110e` | cards and panels |
| `--surface-raised` | `#f9f7ef` | `#141a15` | controls and rows |
| `--ink` | `#0f1712` | `#e3e7dc` | text |
| `--ink-muted` | `#4b564f` | `#a3ae9f` | secondary text |
| `--line` | `#b7bcae` | `#2f3a32` | hairlines |
| `--line-strong` | `#6b766c` | `#5a685d` | control borders, corner ticks |
| `--accent` | `#c0481f` | `#ff7a4a` | primary action, active destination |
| `--signal` | `#2c6b3c` | `#8fdc93` | positive tone |
| `--warning` | `#7a4b0a` | `#f2cf7a` | caution tone |
| `--danger` | `#8f2a20` | `#ff9f92` | negative tone |
| `--navigation` | `#0b0f0d` | `#030504` | destination strip |
| `--navigation-active` | `#c0481f` | `#ff7a4a` | active destination rule |

Radii (`--radius-control`, `--radius-panel`, `--radius-status`) are zero.
`--shadow-panel` and `--shadow-message` are `none` and remain declared so the
contract and the high-contrast media query keep their hooks. `--tick` is the
corner tick length.

## Component treatments

- **Header.** The title block carries a three-pixel accent rule on its leading
  edge. The kicker is tracked technical uppercase; the title is condensed
  uppercase at title-two size.
- **Status strip and chips.** Transparent, hairline-bordered, technical
  uppercase, prefixed by the tone glyph. Counters share the shape without a
  glyph. The unread counter keeps the accent fill.
- **Cards.** Surface fill, hairline border, corner ticks. Rows inside a card
  separate with hairlines only.
- **Buttons.** Condensed uppercase, tracked. Primary actions fill with the
  accent; secondary actions are outlined in ink; disabled actions keep the
  quiet treatment from the mobile UI quality change.
- **Form labels.** Technical uppercase at a size relative to the body so
  Dynamic Type still scales them.
- **Destination bar.** Black strip with a top hairline. Items are technical
  uppercase labels under their icons. The active item shows a two-pixel accent
  rule along its top edge and white ink; there is no lozenge.
- **Scan indicator.** A square bar of accent hazard stripes that travels while
  scanning.

## Contrast

Measured from the final token values. Text pairs must reach 4.5:1 and
hairlines 3:1.

| Pair | Light | Dark |
|---|---|---|
| ink on canvas | 14.05 | 15.92 |
| ink on surface | 15.70 | 15.16 |
| muted ink on canvas | 5.90 | 8.67 |
| muted ink on raised surface | 7.14 | 7.67 |
| signal on canvas | 4.94 | 12.20 |
| warning on warning-soft | 5.81 | 9.83 |
| danger on canvas | 6.42 | 10.11 |
| white on accent (primary action) | 5.01 | 7.31 (accent ink) |
| navigation ink on navigation | 8.90 | 8.39 |
| line on canvas / surface / raised | 3.04 / 3.39 / 3.67 | 3.40 / 3.24 / 3.01 |
| accent rule on navigation | 3.85 | 7.92 |

The hairline token sat near 1.6:1 before this change; it now clears 3:1 on
every surface in both themes, which is also what gives the panels their
framed, drawn-on-paper edge.

## Thread screen

The open conversation is its own screen in compact mode. The application
header steps aside, the thread header carries a chevron back control, the
peer name, and the short hash on one row, the history takes every spare row
and scrolls internally, and the composer is one line tall with the delivery
method beside its label. Sent messages hang right with an accent rule on
their trailing edge; received messages hang left with the signal rule.
Timestamps render as UTC date-time groups with a trailing Z. The composer
shows one status line, never the same sentence twice, and only mentions
propagated delivery when it is selected or unavailable.
