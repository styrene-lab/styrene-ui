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
