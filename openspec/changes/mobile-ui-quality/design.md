# Mobile UI Quality Design

## Findings

Screens were captured on 2026-09-02 from the fixture build on the iPhone 17 Pro
simulator and from the live build on a physical iPhone 17 Pro. The captures
show the same defects on every tab.

| Finding | Cause |
|---------|-------|
| Headings break inside words ("Conversatio / ns") | `h1, h2, h3, p, output { overflow-wrap: anywhere }` in `mobile.css` applies identifier wrapping to all prose |
| Every screen names itself three times (hero, section label, card title) | The shell renders a hero title, a section eyebrow, and a card heading from the same page name |
| The header, session chip, fixture banner, and Operational summary occupy about forty percent of the viewport before content on every tab | The header block and the summary disclosure are repeated per tab instead of living in one status surface |
| Good states, counters, labels, and the selected tab all use the same teal | `--signal` is the brand colour, the eyebrow colour, the default chip colour, and the badge colour; there is no `[data-tone="positive"]` rule, so positive is simply the unstyled default |
| Disabled primary buttons render as a salmon fill that reads as an error | The disabled state reuses the accent fill at reduced opacity, while disabled secondary buttons use a ghost treatment |
| Helper text dangles outside card padding ("New Message is unavailable in this view.") | Availability hints are emitted as sibling paragraphs rather than as part of the control |
| Primary navigation shows letters ("M", "P", "N") | The tab bar has no icon assets |
| The session chip is clipped under a picture-in-picture window | The header has no trailing safe area |
| A Bluetooth pairing request flashes at launch and disappears | The iOS BLE controller connects to the stored approved RNode as soon as it starts, and its fifteen-second deadline cancels the connection, which dismisses the system pairing alert |
| The fixture UI test build did not compile | Two platform types were used unconditionally but imported only without the `ui-test` feature |

## Approach

### One status surface

The shell keeps a compact status strip under a single screen title: brand
mark, screen name, and one session chip with trailing safe-area padding. The
Operational summary moves into a status sheet reachable from the strip and from
More, so tabs start their content within a quarter of the viewport.

### Tones the stylesheet honors

`StatusTone` stays the model. The stylesheet gains explicit rules for all four
tones, a neutral badge style for counters, and reserves `--accent` for enabled
primary actions and the unread indicator. Disabled controls share one
treatment: reduced-contrast text on the surface colour, no fill. Destructive
actions keep the danger tone. Both themes are checked against WCAG AA for text
and 3:1 for control boundaries.

### Scoped wrapping and a mobile type scale

`overflow-wrap: anywhere` applies only to `.identity` and `.technical-value`.
Headings use a type scale sized for a 393-point viewport, with hero text no
larger than 28 points, and cards keep helper text inside their padding under
the control it describes.

### Icon navigation

The tab bar renders bundled SVG icons with labels. Control size remains at
least 44 by 44 points, and the existing minimum-size UI test keeps guarding it.

### Explicit RNode reconnection

`IosBleHost::run` no longer connects at start. It publishes the approved
peripheral as a `Remembered` phase with a reconnect action. A connect in flight
keeps its deadline only until CoreBluetooth reports the connection; after
`didConnect`, service discovery and characteristic setup run without an
application-side cancellation, so a system pairing request is never dismissed
by the application. Cancel is always visible while connecting. A bond that is
not retained between launches is surfaced as a diagnostic rather than hidden
behind a retry.

## Evidence

The UI test suite retains one screenshot per tab in light and dark appearance
for the fixture build. The physical launch capture is repeated after the BLE
change with the RNode in range and not in range.
