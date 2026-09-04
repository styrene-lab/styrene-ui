# Tasks

## 1. Policy

- [ ] 1.1 Add a per-contact delivery preference to the session state with a default of direct-then-node (partial: `DeliveryPreference` and its persistence in `ContactBook`/`PreferenceStore` landed in `contact-centric-shell` 1.1; the thread-header control that sets it and the send-time wiring that reads it landed in 1.2/1.3 below)
- [x] 1.2 Remove the method select from the composer and drive the send from the preference (the per-message `select#mobile.delivery-method` and its `mobile.delivery-method-status` hint are gone; the thread header carries a two-state `mobile.delivery-preference` control instead, and `Composer` derives `requested_method` from the resolved contact's `delivery_preference` prop: `DirectThenNode` always requests `Direct` today, `AlwaysViaNode` requests `Propagated` only when a node is selected and ready)
- [x] 1.3 Show pre-flight truth on the composer's status line from reachability and node readiness (`p#mobile.composer-status` now states what will happen before Send, keyed on the preference and the contact's link, never the generic "Ready to send.")
- [ ] 1.4 Add the long-press override on Send
- [ ] 1.5 Add the app-level unreachable-peer rule to More

Note: the "hand to node when direct fails" fallback described for the
`DirectThenNode` preference is daemon behaviour that does not exist yet.
Today the daemon only falls back direct → opportunistic, so an unreachable
peer with `DirectThenNode` still requests `Direct` (never `Propagated`); the
composer's status line previews the future hand-off ("Peer unreachable.
Will hand to the propagation node.") ahead of the daemon actually doing it.
Closing that gap is out of scope here and belongs with 1.4/1.5.
