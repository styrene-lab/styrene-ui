# Delivery Policy

## Intent

Stop asking the operator to predict reachability before every message. The
daemon already knows whether a peer can be reached now and already falls
back; the composer's per-message method select is a control nobody needs to
touch, taking a row of the thread.

## When an operator would choose

| Method | The case where it is the right answer |
|---|---|
| Direct | The peer is reachable now and proof of delivery matters: a receipt means it landed on their device. Time-critical coordination. |
| Propagated | The peer is offline or intermittent, or the operator is about to go dark, or the link is thin enough that one hand-off to a node costs less airtime than a link handshake and retries. |
| Opportunistic | Never a human choice; a transport fallback for small messages. |

Direct whenever it can work, propagated whenever it cannot. That is a policy,
not a per-message decision.

## Shape

- A per-contact preference, set in the contact sheet or thread header: "deliver now, hand to node if unreachable" (default) or "always via node".
- One app-level rule for an unreachable peer with no node selected: hold and retry, or fail fast.
- A rare per-message override behind a long-press on Send: "via node this once".
- The composer's one status line does pre-flight truth: "will deliver now", "will hand to node", or "no node selected".
- The card records the method that carried the message, as delivery details already do.

## Scope

The composer, the thread header, the contact sheet from `contact-centric-shell`,
and the session's per-peer preference. The daemon's fallback behaviour is
unchanged.

## Success criteria

- The composer has no method control; the status line states what will happen before Send.
- A contact's preference persists and is applied to every send to that contact.
- The override is reachable but never in the way.
