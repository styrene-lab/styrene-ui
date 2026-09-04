# Tasks

## 1. Policy

- [ ] 1.1 Add a per-contact delivery preference to the session state with a default of direct-then-node (partial: `DeliveryPreference` and its persistence in `ContactBook`/`PreferenceStore` landed in `contact-centric-shell` 1.1; the composer, thread header, and send-time wiring to this preference remain)
- [ ] 1.2 Remove the method select from the composer and drive the send from the preference
- [ ] 1.3 Show pre-flight truth on the composer's status line from reachability and node readiness
- [ ] 1.4 Add the long-press override on Send
- [ ] 1.5 Add the app-level unreachable-peer rule to More
