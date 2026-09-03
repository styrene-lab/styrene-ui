# Mobile Design Corpus - Delta Spec

## ADDED Requirements

### Requirement: Surfaces are hard-edged and unelevated

Every radius token must resolve to zero, no component may use a drop shadow,
and cards must carry hairline borders with corner ticks in the strong line
colour.

#### Scenario: Card rendering
Given any surface card in either appearance
When it is rendered
Then its corners are square
And it has no shadow
And each corner shows a tick in the strong line colour

### Requirement: Machine-produced values use the technical face

Hashes, counters, ages, phases, state labels, kickers, and form labels must
render in the technical face. Headings, chosen names, and body copy must use
the interface face.

#### Scenario: Status strip
Given a connected session
When the header is shown
Then the session label renders in the technical face in uppercase
And the screen title renders in the interface face

### Requirement: Tone carries a glyph

Positive, caution, negative, and neutral tones must be prefixed by `●`, `▲`,
`✕`, and `○` respectively, drawn by the stylesheet from the tone attribute, in
addition to their distinct colours.

#### Scenario: Caution state without colour
Given a caution-toned chip viewed in a monochrome rendering
When it is compared with a positive-toned chip
Then the two are distinguishable by their leading glyphs alone

### Requirement: Contrast holds in both appearances

Every text colour against the surface it is placed on must meet a 4.5:1
contrast ratio, and every hairline against its surface must meet 3:1, in both
the light and dark palettes.

#### Scenario: Muted text on canvas
Given the muted ink token and the canvas token of either appearance
When their contrast ratio is measured
Then it is at least 4.5:1
