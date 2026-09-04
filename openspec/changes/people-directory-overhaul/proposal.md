# People Directory Overhaul

> Superseded by `contact-centric-shell`: the roster search, filter, and sort tasks move to the Network directory; People becomes Contacts.

## Intent

Turn the People screen from a dump of every announce the network relays into
a directory an operator can use: people they talk to first, everything else
findable on demand.

## Problem

On a live node connected through a hub, People lists every announced
destination the hub has ever relayed, near 250 entries at last count and
1,478 on a busy hub. Propagation nodes, NomadNet nodes, and delivery
destinations share one list. Order is arrival order. A peer the operator
just connected to directly is not distinguishable from a relay that
announced a day ago, and a peer that never announces does not appear at
all, so the operator has no way to reach it except by pasting its hash into
the New Message form.

## Scope

The People screen and its state in `styrene-ui-app`, the peer projection
in the mobile session, and any daemon queries those need. It keeps the
roster row treatment from the field-kit corpus.

It excludes the daemon's announce policy, transport behaviour, and the
desktop application.

## Shape

- **Two sections, not one list.** Contacts: peers with a conversation, a
  saved name, or a pinned entry. Discovered: everything else, collapsed by
  default, with a count.
- **Aspect is a filter, not a tag.** Delivery destinations by default;
  propagation and NomadNet nodes behind a toggle.
- **Search that finds things.** Name, hash prefix, and aspect, with the
  result count in the heading; the filter field already in place becomes
  this.
- **Sort the operator can read.** Most recent announce first by default;
  name and announce count as alternatives.
- **Reach a peer that has not announced.** A "Message a destination" entry
  at the top of People that takes a 32-hex hash, requests its path, and opens
  the thread, so a freshly connected peer is reachable without discovering
  it first.
- **Show how a peer was seen.** Which interface carried the last announce,
  so a direct TCP peer reads differently from a hub relay.

## Success criteria

- A node with 1,478 announces opens People in under a second and shows contacts first.
- Typing four hex characters narrows the list to the matching peers, with the count shown.
- An operator can open a thread to a destination that has never announced from the People screen.
- The existing roster row, filter identifier, and capture test keep working.
