# Desktop workspaces

The desktop opens on **Overview**. The left navigation shows Overview, Messages,
Browse, Network and Fleet, with Lab and System grouped as tools. Below 900 CSS
pixels, use **Navigation** to open the destination drawer.

Overview shows current session/profile context, saved interface states and
reported connection errors. Selecting a saved interface opens Network's interface
manager. TX is queued traffic, not delivery confirmation. Overview shows at most
five recent operator events; Activity retains broader runtime diagnostics.

**Messages → Conversations** retains chat. **Messages → Delivery** contains the
existing propagation queues and synchronization diagnostics. Delivery keeps
Messages selected in primary navigation.

**Browse** contains the existing NomadNet page hosts, address bar, page navigation,
forms and downloads. Loaded page/transfer state remains in application stores
when changing workspaces. Saved bookmarks and external I2P/Yggdrasil browser
handlers are future work; the shell does not advertise them as available.

Rail summaries distinguish unavailable/loading data from reported counts. Fleet
summarizes active jobs because discovered Styrene nodes do not establish a managed
roster. Browse counts advertised page hosts, not proven reachable sites. Paginated
unread counts identify when only the loaded conversation list is counted.

Launch route hints accept `overview`, `messages`, `messages/delivery`, `browse`,
`network`, `fleet`, `lab` and `system` through `STYRENE_DX_ROUTE`. Legacy `command`,
`content` and `propagation` hints map to the corresponding new workspace. Existing
`network/interfaces`, `network/routes`, `network/links` and `network/map` hints remain.

See the [workspace specification](../openspec/changes/desktop-workspaces/proposal.md)
and [verification record](../openspec/changes/desktop-workspaces/verification.md).
