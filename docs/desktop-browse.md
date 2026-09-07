# Browse discovery and page waits

Browse lists announced NomadNet page hosts. An announcement names a service; it
is not a successful reachability check. A route in the daemon's path table means
that a path was recorded, not that the destination will establish a link now.

The directory supports case-insensitive name/address search, stable name ordering,
and 20 initial entries with an explicit option to show more. Local pages are under
**On this node**. The directory shows announcement age and either route evidence
or the last attempt in this session. It retains at most 128 recent node outcomes.
Changing backend sessions clears these outcomes. Search does not probe remote hosts.

Opening a page starts a separate UI task. Another selection replaces the local
wait; **Stop waiting** ends it. A local deadline ends the wait after 45 seconds.
Neither action claims to cancel work already accepted by the daemon. Normal
commands and interface refreshes are no longer held behind the UI page await.
Provider and transport scheduling can still affect response time.

The current daemon returns page-stage evidence with its response, rather than
streaming it during the request. Loading therefore identifies the destination and
wait budget without inventing live stage progress. A local timeout cannot identify
which remote stage failed. When an authoritative failure arrives within the budget,
the main view names its stage; **Request details** contains the technical evidence.
Raw provider error strings remain redacted.

A cached path can coexist with a failed link. For example, successful path discovery
and identity resolution followed by a link timeout does not establish that the host
is permanently offline. It establishes that this attempt did not establish a link.

Follow-up backend work should provide stage events and explicit cancellation before
the UI offers stage-specific progress or remote cancellation. A longer, operator-
selected wait for slow links needs its own policy; this first pass uses 45 seconds.
No daemon protocol, identity lifecycle, or mobile implementation changed here.

The local budget includes the default daemon operation (30 seconds) and IPC
cleanup allowance (15 seconds). It must not discard the daemon result at the
earlier 15-second observation cutoff. Stop waiting remains available immediately.

## Directory controls

Use All hosts, Favorites, or Attempted to choose a collection. Attempted means an
attempt recorded in the current backend session, including failures. Search
matches names and destination addresses. Filters & sorting offers announce age
(past hour/day/week) and name/newest ordering. Missing or future timestamps do
not qualify for an age window. Reset clears the query, collection, age and sort;
it does not remove favorites. Results remain limited to 20 until Show more.

Star a host to save its address/name locally. Favorites persist in
`$XDG_CONFIG_HOME/styrene-ui/browse-favorites.json`, falling back to
`~/.config/styrene-ui/browse-favorites.json`. The list is shared by desktop backend
sessions on this OS account, contains no identity material, and is capped at 256
entries. A saved host absent from discovery remains visible with unknown announce
time. Saving it does not establish a route or offline availability. Storage errors
are displayed; an unreadable existing list blocks edits to avoid overwriting it.

I2P and Yggdrasil are explicitly unavailable source placeholders. Cached content
is disabled until a provider exposes a cache inventory. No mock hosts are mixed
into the live directory and no filtering action sends network probes.
