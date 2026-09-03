# Composer Sending State

## Intent

Show the operator that a send is in flight from the moment Send is tapped
until the backend has recorded the attempt, and refuse a second submit in
between.

## Scope

The mobile composer in `styrene-ui-app`: a sending flag keyed to the backend
generation at submit, the Send control's label and busy style while it is
set, and a status sentence. Nothing in the daemon or session changes.

## Success criteria

- Tapping Send changes the control to "Sending…" and disables it until the next backend snapshot.
- The status line says the message will appear once the backend accepts it.
- A render without a submit shows the control as not sending.
