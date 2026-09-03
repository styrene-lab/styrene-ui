# Design

## Propagation board

The section heading row carries the readiness chip. Its tone follows the
readiness: ready is positive, unselected is neutral, unavailable is negative,
inactive and invalid metadata are caution. The selected node prints on one
technical line. The node select and the synchronization action share a field
row; the label stays for assistive technology only. The policy, automatic
synchronization, readiness sentence, trigger capabilities, active trigger,
last synchronization, cooldown, and airtime sentences live behind a "Policy
and evidence" disclosure. The status region stays visible under the row. Every
identifier is unchanged, and the synchronization action still describes
itself by the airtime sentence inside the disclosure.

## Dynamic Type

Rows on the boards set their sizes from system text styles: headline for
bearer names, body for roster names, caption two for technical metadata,
footnote for previews, definition grids, and panel sentences, caption one for
permission values. Chips, counters, and the status strip wrap inside their
container instead of overflowing it.

## Bluetooth, recovery, New Message

Spacing tightens: the candidate list and rows lose their card padding, the
recovery forms and restore actions share a half-rem gap, and the New Message
peer rows align with the roster.
