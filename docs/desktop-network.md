# Desktop Network workspace

Network opens on the peer inventory. Interfaces, Routes, and Links have their
own tables and search. Map is an optional topology view. The Connection menu in
the app header contains runtime profile and socket configuration.

Select a row to inspect it. The inspector retains full identifiers and copy,
relevant observations, and authorized actions. Peer and route selection supplies
the destination; link selection also supplies the link identifier. Native request
fields stay in a separate disclosure. Advanced targeting remains available for
operations against an explicit identifier. Destructive actions retain the typed
confirmation and generation checks.

The page Actions control exposes node-level operations such as Announce.
Activity groups submitted operations, native requests, and resource transfers.
Empty categories do not occupy the initial workspace.

## Observation semantics

- Peer discovery, route observations, and link lifecycle are independent.
  Routes and links remain visible when the peer has not been discovered.
- A recent announcement does not prove reachability. The peer table distinguishes
  an active link, a known current route, and reachability not established.
- Missing timestamps and RTT values show Unknown. An unreported peer status
  does not match either the Online or Offline filter.
- A disconnected session shows unavailable observations, not an empty network.
- Map destinations without discovery metadata have unknown status. Map edges
  retain interface, route, and active-link categories; discovery alone creates
  no edge. Stale routes and inactive links do not become current topology edges.

The map starts with a static layout. Fit to view and zoom controls are visible;
animation is an explicit setting. Map selection uses the same inspector as the
inventory. On narrow windows, the inspector replaces the inventory and Close
details returns to it.

## Reproducible entry points

Set `STYRENE_DX_ROUTE` when launching to open `network`, `network/interfaces`,
`network/routes`, `network/links`, or `network/map`. This only chooses a view.
Runtime selection still uses `STYRENE_DX_PROFILE` and its normal configuration;
view selection never creates a Fixture or replaces a failed Live session.

Run desktop tests with `cargo test --locked -p styrene-dx --bin styrene-dx`.
Native screenshots are separate evidence from component tests. The Comms Lab
Mac run records are under `lab/runs/network-recomposition/` in its coordination
checkout. Physical radio and multi-node delivery are outside these UI checks.

## Local connectivity map

The map opens on the local node, up to 16 transport interfaces and their reported
connected endpoints, and up to eight active links. Remote destinations collapse
into groups by their observed route interface. Discovery without a current route
and routes with an unknown interface have separate groups. These groups are
presentation summaries; they do not represent physical machines. A TCP endpoint
is not assigned a mesh identity without evidence.

Select a group on the map or in Explore to reveal 16 destinations per page and
their reported next hops. Search scans destinations across all groups and shows
bounded result pages. Select Local overview and clear search to collapse again.
The page's peer-specific filters apply to the peer inventory, not the map.

At most 100 nodes and 200 relationships reach the renderer. At most 16 group
summaries appear on the canvas; the selector retains every group. The UI reports
omitted interfaces, links, and groups. Stale routes and inactive link records are
excluded; offline discovery alone is excluded, while a current route or active
link remains independent evidence. Historical counts and the inventory tabs
retain access to the complete observations.

The daemon keeps its routing state. This change bounds layout and rendering,
not IPC inventory size. Structural updates reconcile positions; traffic counters
do not rebuild the graph. Animation remains opt-in. These limits are initial
presentation budgets, not measured frame-rate guarantees. Linux runtime
validation remains separate from the Mac build and capture.
