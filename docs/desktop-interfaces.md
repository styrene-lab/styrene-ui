# Manage desktop interfaces

Open **Network → Interfaces** in a Live or Embedded session. The daemon must
advertise `runtime.interface-management.v1`, and the operator needs
`rpc.config_update` permission to change configuration.

Select **Add interface** to create a named TCP client or TCP listener. A client
connects to a remote host and port; a listener binds a local IP address and port.
For the community hub, use a TCP client with host `rns.styrene.io` and port
`4242`. Saving with **Enable now and on daemon startup** checked starts the
interface. Saving disabled retains it without starting a connection.

Select a row to inspect its configuration, endpoints, traffic, runtime state,
and, for a listener, accepted connections. **Edit**, **Enable / Disable**,
**Reconnect / Restart listener**, and **Delete** act on the selected interface.
Deletion requires confirmation. Applying an edit restarts an enabled interface;
stopping or restarting a listener also closes its accepted connections.

Configuration is saved before runtime changes. A failed connection remains in
the list with its runtime state; TCP workers retry automatically. A failed save
does not change the running interface. Concurrent stale edits are rejected.
Disabling or deleting all listeners remains effective after daemon restart.
On the first change, existing command-line TCP listeners are adopted into saved
configuration. Subsequent startups use that managed interface list.

This editor currently supports TCP clients and listeners. RNode entries may be
reported, but their radio settings are not editable through this contract.
Fixture sessions and older daemons show why management is unavailable. Mobile
interface management is not enabled by this desktop feature.
