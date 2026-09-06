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
wait; **Stop waiting** ends it. A local deadline ends the wait after 15 seconds.
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
selected wait for slow links needs its own policy; this first pass uses 15 seconds.
No daemon protocol, identity lifecycle, or mobile implementation changed here.
