# Tasks

## 1. Model

- [x] 1.1 Define the contact projection in the session: name, destinations, capabilities, link modes, presence, preference (`Contact`, `ContactRole`, `LinkMode`, `DeliveryPreference`, `ContactBook`, and `project_contacts`/`contact_lists` landed in `styrene-ui-state`; `Peer` carries `identity_hash`/`hops`/`interface_kind`; the contact book persists through a new `PreferenceStore` in `styrene-ui-platform`, backed by `NSUserDefaults` on iOS and in-memory elsewhere; `MobileActionKind` gained `ToggleFavourite`/`ToggleBookmark`/`SetAlias`/`SetDeliveryPreference`, wired through the mobile session)
- [x] 1.1a Group announces by identity and derive roles from aspects: person, page host, relay, tunnel peer, unknown (`project_contacts` groups by `identity_hash`, falling back to `destination_hash`; `ContactRole::from_aspect` derives roles)
- [x] 1.1b Offer verbs from roles only: Message for a person, Browse for a page host, Use as relay for a relay; no composer for an identity without a person role (contact rows render the verb their roles allow: a person's row opens or starts a conversation, a page host's row offers a disabled Browse until a viewer exists, and a relay or unknown identity renders as a plain row with no verb; only Messages carries a composer, and only Person contacts reach it)
- [ ] 1.2 Expose a link-mode summary per peer from the daemon: RNS path and hops, direct reachability, tunnel state

## 2. Screens

- [x] 2.1 Messages: conversation rows from contacts with last line, time, and link-mode glyph (`ConversationList` takes the contact projection and renders `span.link-mode` with `data-link` and a named mode after each name; the thread header carries the same glyph in `span.thread-link` with "RNS · 1 hop", "via node", "tunnel", or "no path")
- [x] 2.2 Contacts: replace People with active, reachable, and known sections over messaged or favourited people, plus add-by-destination (section `mobile.contacts` renders `contact_lists().contacts` in Active, Reachable, and Known groups; `mobile.contact-destination` plus `mobile.contact-add` dispatch `StartConversation` on a valid 32-character hash and open the thread)
- [x] 2.2a Pages: recent and bookmarked page hosts with the browser as the primary action (section `mobile.pages` groups Bookmarked and Recently seen, newest first and capped at twenty; Browse is rendered disabled with a hint because mobile has no page viewer yet)
- [x] 2.2b Favourite on every directory entry, routed to Contacts or Pages by role, with removal that leaves history alone (`mobile.contact-favourite.<id>` and `mobile.contact-bookmark.<id>` on the operator's own lists, `mobile.directory-favourite.<id>` and `mobile.directory-bookmark.<id>` in the directory; both dispatch a toggle that only edits the contact book)
- [ ] 2.2c Persist favourites, messaged people, and browsed hosts in the session store
- [x] 2.3 Network: move the announce directory here with search, aspect filter, and sort (a `details` card `mobile.directory` under Propagation holds every announce grouped by primary role as People, Page hosts, Relays, and Other, each with a count, behind the shared `mobile.directory-filter`; the People screen no longer carries the firehose)
- [ ] 2.4 Thread header and message cards: show the link mode and reachability

## 3. Evidence

- [ ] 3.1 Capture every screen on a fixture with mixed link modes in both appearances and at the largest text size
