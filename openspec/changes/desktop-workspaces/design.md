# Design

Use existing AppRoute and DomainStores with a small pure navigation projection.
Keep Delivery as an internal route whose primary parent is Messages. Build one
summary rail component and one Overview component; no plugin framework changes.
Use existing data-state/capability evidence, never an inferred Fleet roster from
all discovered peers. Counts derived from loaded pages must say loaded.

Use text and consistent stroke icons, neutral summaries, accent selection and
explicit error text. The rail becomes a labeled drawer on narrow windows.
The header retains existing connection controls until managed profile APIs are
integrated; do not rename an execution mode as a saved profile.

Browse keeps the current page and transfer in app-owned stores. Bookmarks and a
cross-network service directory need their own persistence contracts and are
excluded from this shell change. No Identity checkout changes are required.

Validate projections and route compatibility with Rust tests; run the existing
desktop corpus, update its route names, then build using just mac-app. Capture
fixture and Live surfaces with artifact/process identity; report unavailable
platform evidence separately.
