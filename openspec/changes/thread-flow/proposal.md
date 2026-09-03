# Thread Flow

## Intent

Make the open conversation read as a conversation: newest message beside the
composer, delivery detail out of the way until asked for, and a send that
reads as in flight only while it is.

## Scope

The message history order and anchoring, the message card's delivery detail
layout, and the composer's in-flight rule in `styrene-ui-app`. The backend
pin moves to the styrene-rs main that announces on the connected-state
transition.

## Success criteria

- The newest message sits at the bottom of the history, next to the composer, and the history opens scrolled there.
- A message card shows its body, its state, and a failure reason; requested and actual method, fallback, and evidence sit behind one "Delivery details" disclosure.
- "Sending…" clears when a new outbound record appears or after a bounded wait, never later.
