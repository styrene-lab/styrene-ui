# Browse discovery and request feedback

## Intent

Replace misleading green availability and unbounded-looking waits with searchable
advertisement evidence, recent attempt outcomes and bounded, replaceable reads.

## Scope

Desktop directory, local request lifecycle and existing page diagnostics. No new
reachability probes, backend cancellation contract or Identity work. Remote reads
use a 45-second local wait budget; slower transports may require a later explicit
long-wait policy. A local timeout does not prove the node is permanently offline.
