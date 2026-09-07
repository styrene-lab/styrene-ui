# Network peer directory

Make discovery usable with a large announce inventory: compact controls, labelled
filters, sortable semantic table columns, and 50 destinations per page. Start
with newest announces. Search names, destinations, identity hashes and capabilities.
Keep role, reported status, route evidence, route interface and announce age distinct.
An announce or known route must never imply successful page delivery.

Keep separate destination records: identical names alone do not establish a shared
identity. Retain full identifiers and capability metadata in the selected record.
Move activity identifiers and relationships into disclosure panels. Show bounded
recent observations with status, progress and styled cancellation controls while
preserving existing authorization and confirmation. These are last reported
observations, not a new liveness protocol.

Backend contracts and mobile source stay unchanged. Shared navigation and filter
semantics can be adopted by mobile independently; this change validates macOS only.

## Peer interaction correction

Clicking anywhere in a table row selects it; the peer-name button remains the
keyboard entry point. Selection opens contextual operations for peers as well
as routes and links. Message and Browse precede collapsed discovery metadata.
Path requests and link opens target the selected destination. Probe and close
use a known, non-stale active link when one is present. Authorization and
confirmation remain enforced by the existing operation controls.
