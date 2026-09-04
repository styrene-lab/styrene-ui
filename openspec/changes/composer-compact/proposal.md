# Composer Compact

## Intent

Give the conversation back to the conversation. The composer takes one
line for the draft and an icon for Send, one quiet line for the delivery
method, and says nothing unless something blocks the send.

## Scope

The composer markup and stylesheet in `styrene-ui-app`. Identifiers and
the delivery select are unchanged.

## Success criteria

- Send is a single touch-target icon whose accessible name is "Send", or "Sending" while in flight.
- The Done control is gone; the page's tap-outside listener dismisses the keyboard.
- The delivery method is one small technical control under the field with its label for assistive technology only.
- "Ready to send." is never shown; the status line appears only for a blocking or explanatory condition.
