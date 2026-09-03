# Design

## Lock screen

While the App Lock gate is closed the shell root carries `data-locked="true"`
and renders one `lock-screen` section: brand kicker, a "Locked" title whose
leading glyph is `○` while waiting and `✕` after a closed outcome, one status
sentence, an Unlock action, the custody assurance, and the diagnostic code.
Every other child of the shell is hidden by the stylesheet. The Unlock action
keeps the `mobile.app-unlock-retry` identifier, is disabled while the system
sheet is up or when no retry handler is attached, and describes itself by the
`mobile.session-failure` element that the closed outcome renders.

Status sentences by outcome:

| Outcome | Sentence |
|---|---|
| authenticating | Waiting for Face ID or the device passcode. |
| `app_unlock_cancelled` | Unlock was cancelled. |
| `app_unlock_unavailable` | Device authentication is unavailable on this device. |
| `app_unlock_failed` | Device authentication failed. |

Failures that are not App Lock keep the failure banner.

## People roster

A search field above the list filters by display name, hash, or aspect,
client-side, and the count badge reads "n of total" while a filter is active.
Rows are sorted by observation age, newest first. The roster is one framed
list with hairline rows; each row shows the hash glyph, the name with the
short hash beside it, and one technical line: aspect label, age, announce
count. Aspect labels: `lxmf.delivery` is LXMF, `lxmf.propagation` is
Propagation, `nomadnetwork.node` is NomadNet; anything else prints as is.
Ages print as seconds under a minute, then minutes, hours, and days. The row
action keeps its accessible name and renders as a chevron.

## Conversation list

Each row shows the name, short hash, and the UTC time of the latest message on
one line, then a single-line preview of that message. The preview is the
latest message for the peer by timestamp, cut at 120 characters.

## Network

The bearer board opens the screen: one card, one hairline row per bearer with
its tone chip. The TCP endpoint editor and the display-name editor lay their
label, input, action, and hint on a grid so the input and its button share a
line. Bluetooth actions sit side by side under the status line.

## More

Definition lists in settings cards and the operational summary render as a
two-column grid with technical-face terms. Permission and custody values are
technical uppercase with a tone glyph drawn from their state attribute:
`●` granted or available, `✕` denied or restricted, `▲` unavailable, `○`
otherwise.
