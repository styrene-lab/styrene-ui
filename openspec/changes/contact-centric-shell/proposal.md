# Contact-Centric Shell

## Intent

Reorganise the mobile shell around the peer and the quality of the link to
them, the way a communications application does, rather than around the
transport's firehose. Nothing browseable is hidden; it is ordered by what the
operator acts on.

## Why now

Styrene will not stay RNS/LXMF-bound. Direct IPsec tunnels between peers,
negotiated over RNS/LXMF and carried over a TCP path where one exists, are
on the roadmap and partly implemented. Once a peer can be reached over an
RNS path, through a propagation node, or over a tunnel, method, aspect, and
transport are properties of the contact, not of a message or a screen.

## Model

A **contact** is an identity with:

- a name: the operator's alias over the announced name
- its destinations and what it can do: delivery, node, propagation
- the link modes available to it right now: RNS path with hop count, direct TCP reachability, tunnel up
- when it was last seen, and whether it is reachable now
- the operator's delivery preference for it (see `delivery-policy`)

Messages, the contact sheet, and the thread header all render from that
object.

## Roles

A destination announces an aspect, and the aspect decides what the operator
can do with it. A NomadNet page is not something to send a message to.

| Role | Aspect | Verbs |
|---|---|---|
| Person | `lxmf.delivery` | Message, with receipts. The only role that belongs in Messages and Contacts by default. |
| Page host | `nomadnetwork.node`, Styrene page host | Browse pages and files. |
| Relay | `lxmf.propagation` | Select as propagation node, sync. |
| Tunnel peer | tunnel capability (roadmap) | Open or close a tunnel; message over it only if the identity also has a person role. |
| Unknown | anything else | Show the raw aspect; offer no verb. |

One identity often announces several destinations, so the contact is the
identity and its roles are the set of aspects seen for that identity. The
projection must group announces by identity before anything renders. The
primary action follows the roles: a person gets Message, a page host gets
Browse, a relay gets Use as relay, and an identity with several roles gets
each. An identity with no person role never shows a composer and never
appears in Messages.

Where each role lives: Contacts holds identities with a person role or that
the operator saved; the Network directory holds everything, sectioned by
role, with person entries offering "add to contacts"; the thread header
notes when the person's identity is also a relay in use.

## Screens

- **Messages**: conversations sorted by activity, one row per contact with name, last line, time, and a link-mode glyph.
- **Contacts** replaces People: active links first (tunnel or direct), then reachable, then known but unreachable. Add from the directory or by destination hash here.
- **Network** keeps the firehose on purpose: bearers, tunnels, propagation, and a browseable directory of every announced node and page with search, aspect filter, and sort. The People directory overhaul's roster work moves here.
- **Thread header** states the link, not the method: "RNS, 1 hop", "tunnel", or "via node", with reachability. The composer's status line does pre-flight.
- **Message cards** record the link mode that carried them alongside the delivery details.

## Scope

The shell's information architecture and the session's contact projection
in `styrene-ui`, and whatever daemon queries the link-mode summary needs.
It supersedes `people-directory-overhaul`, whose search and filter tasks
carry over to the Network directory.

## Success criteria

- Messages and Contacts show only peers the operator has a relationship with, ordered by activity and by link quality respectively.
- Every announced node and page stays reachable through the Network directory with search and aspect filters.
- A contact's link modes and reachability are visible in Contacts, the thread header, and the message cards.
- The model has room for a tunnel link mode without a second redesign.
