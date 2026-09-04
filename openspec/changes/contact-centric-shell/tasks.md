# Tasks

## 1. Model

- [x] 1.1 Define the contact projection in the session: name, destinations, capabilities, link modes, presence, preference (`Contact`, `ContactRole`, `LinkMode`, `DeliveryPreference`, `ContactBook`, and `project_contacts`/`contact_lists` landed in `styrene-ui-state`; `Peer` carries `identity_hash`/`hops`/`interface_kind`; the contact book persists through a new `PreferenceStore` in `styrene-ui-platform`, backed by `NSUserDefaults` on iOS and in-memory elsewhere; `MobileActionKind` gained `ToggleFavourite`/`ToggleBookmark`/`SetAlias`/`SetDeliveryPreference`, wired through the mobile session)
- [x] 1.1a Group announces by identity and derive roles from aspects: person, page host, relay, tunnel peer, unknown (`project_contacts` groups by `identity_hash`, falling back to `destination_hash`; `ContactRole::from_aspect` derives roles)
- [ ] 1.1b Offer verbs from roles only: Message for a person, Browse for a page host, Use as relay for a relay; no composer for an identity without a person role
- [ ] 1.2 Expose a link-mode summary per peer from the daemon: RNS path and hops, direct reachability, tunnel state

## 2. Screens

- [ ] 2.1 Messages: conversation rows from contacts with last line, time, and link-mode glyph
- [ ] 2.2 Contacts: replace People with active, reachable, and known sections over messaged or favourited people, plus add-by-destination
- [ ] 2.2a Pages: recent and bookmarked page hosts with the browser as the primary action
- [ ] 2.2b Favourite on every directory entry, routed to Contacts or Pages by role, with removal that leaves history alone
- [ ] 2.2c Persist favourites, messaged people, and browsed hosts in the session store
- [ ] 2.3 Network: move the announce directory here with search, aspect filter, and sort
- [ ] 2.4 Thread header and message cards: show the link mode and reachability

## 3. Evidence

- [ ] 3.1 Capture every screen on a fixture with mixed link modes in both appearances and at the largest text size
