# Minimal boundary and lifecycle experiment

## Decisions for the first slice

The extension is a trusted optional Rust dependency compiled with the host's
Dioxus version, currently `=0.8.0-alpha.1`. Independent ownership and versioned
Git consumption do not imply independent runtime installation or a stable Rust
binary ABI. No dynamic library loading is part of this design.

Start with a private registration module and one concrete extension adapter.
Do not publish a generic SDK crate or make a trait for each lifecycle verb until
the first implementation demonstrates a need. The extension's public page can
live in an independently maintained UI crate; its final location is an ownership
handoff, not permission to edit the active Identity checkout.

The proposed registration record needs only:

- A stable extension ID and build version for diagnostics and saved enablement.
- A host contract revision for explicit compatibility checks, not a general resolver.
- One page ID/title and a typed Dioxus component entry point.

The host supplies an Identity client and session context. Actual compatibility is
checked during application assembly; a mismatched descriptor is rejected before
activation. Feature compilation determines whether the extension is bundled.
Service capability discovery handles platform and provider availability at runtime.
Do not introduce a second target/dependency manifest for this slice.

The client exposes public summaries/capabilities and the first read-only request.
Agree its exact DTOs with the Identity owner. Do not expose root-secret access,
arbitrary daemon requests, raw storage paths, global app state, or CLI execution
as the client contract. Lack of an agreed provider uses a mock in the spike and
reports unsupported in production; it must not manufacture a Styrene Identity ID.

Typed interfaces constrain intended access, but trusted compiled Rust shares the
application process. Neither an interface nor a Dioxus error boundary is a sandbox
or protection against every panic, process crash, or malicious dependency.

## Composition and ownership

The host owns the application shell, top bar, page placement, navigation, session
selection, theme tokens, and minimal extension enable/disable controls. It keeps
ordinary profile selection and identity operating summaries available without the
advanced extension. The extension owns the content and temporary view state of
its Identity management page. Use existing UI components and platform-service
contracts; do not add global CSS resets or direct DOM takeover.

Provider authorization, custody, derivation, and identity lifecycle belong to the
Identity/backend owners. CLI and UI consume typed outcomes from the same service
contract. UI mounting must never unlock a provider or mutate custody implicitly.

One dedicated Identity page is sufficient. Do not add a general menu/toolbar/slot
contribution language, arbitrary nested routes, or extension-to-extension messaging.
A small host adapter may match the known page ID and mount its component normally.

This complements `portable-mesh-profile-controls`: the mesh owns profiles,
activation and portable workflow orchestration; advanced Identity management can
open the extension page. Its absence must not remove basic mesh operation. The
extension does not acquire ownership of the active daemon merely by being enabled.

## Lifecycle behavior to dogfood

Keep separate facts rather than one combinatorial state enum: bundled/compatible,
enabled, page mounted, provider availability, and any operation being observed.
Host activation prepares a client/subscription scope; page mount renders it.
Closing the page is not disabling the extension or stopping a provider.

| Event | First-slice behavior | Evidence to collect |
|---|---|---|
| Build excludes extension | No management-page registration; core mesh stays available | Build/test with dependency feature off |
| First run with extension bundled | Enabled by default; version and state inspectable | Registration and settings initialization |
| Enable | Create one activation scope and register one page; no custody mutation | Repeated enable is idempotent |
| Open/close page | Mount/unmount view state; release view subscriptions | Navigation and focus return; no accumulating listeners |
| Provider unavailable | Keep page available with a specific reason and explicit retry | No automatic unlock or fallback identity |
| Provider returns recoverable error | Show error within page; retry without restarting mesh | Structured error and shell still usable |
| Session/identity changes | Invalidate old scope, clear sensitive drafts, bind new client | Old-generation results never reach the new session |
| Disable | Remove entry, navigate out if needed, dispose UI scopes and subscriptions | Late events ignored; enable preference persisted |
| Re-enable | New activation generation, fresh provider query | No duplicate page or replayed transient request |
| App shutdown/restart | Dispose owned UI work; restore enable preference only | No unlock, mutation replay, or persisted secret draft |
| App update excludes extension | Retain harmless preference; show no stale registered page | Core navigation still works |

For this experiment, store only the enabled preference in host app settings,
keyed by extension ID; it is independent of the active mesh profile. Never put
keys, authentication tokens, provider sessions, or pending secret input there.
Moving preferences in portable profiles is deferred.

Backend operations outlive views when the service contract says they do. Before
adding the first mutation, the host must retain an operation ID and outcome route
that remains observable after page closure or extension disable. If that cannot
be guaranteed, block disable for that operation with a precise reason. Do not
implement an operation registry solely for the read-only spike.

Cancel a local subscription or read when supported; otherwise ignore its eventual
result. Cancellation of observation does not cancel backend work. Revoking a
capability prevents new requests; accepted operations follow the provider's own
cancellation/continuation policy. Define that policy before enabling mutations.

## Explicitly deferred plugin lifecycle decisions

Use Identity dogfooding to answer these before widening the boundary:

| Question | Current limit / trigger for revisiting |
|---|---|
| Install/remove code at runtime? | Bundled code only; enabling is a preference. Revisit when independent runtime delivery is required. |
| Hot unload or upgrade? | Application restart/update replaces compiled code. Revisit with actual runtime loading. |
| Dependency resolution and ordering? | One extension, Cargo build graph; no runtime dependency solver. |
| Permissions and user consent? | First request is public/read-only; backend authorization still applies. Define capability-specific interaction before the first privileged operation. |
| Crash containment? | Handle reported errors, not arbitrary native crashes. Isolation needs a separate process/Wasm/web-surface decision. |
| Extension data migrations? | Only host-owned enable preference now. Specify schema, migration, rollback and ownership before adding persistent extension data. |
| Uninstall data retention? | Disable does not delete anything. Future remove/reset must distinguish UI settings, provider credentials, profile storage, and remote trust. |
| Pending operations across update/restart? | No mutation replay. Define durable operation lookup and compatibility before lifecycle mutations. |
| Multiple extension UI surfaces? | One page until a concrete second contribution needs another slot. |
| Mobile lifecycle? | Shared contract plus target-specific adapters. Test suspend/resume and interrupted authentication before claiming mobile readiness. |

## Implementation gate and evidence

First implement registration/mounting with a mock Identity client and a disposable
read-only overview. Exercise the scenarios in the delta spec before adopting a
real provider. Record exact UI/backend/Identity revisions and enabled features.
Compilation, fixture tests, macOS runtime, Linux runtime, and Apple/Android device
acceptance remain separate. Do not claim cross-platform acceptance from the Mac.

After the dogfood, record which fields/hooks were used, where ownership was
ambiguous, cleanup behavior, and the smallest changes needed. Generalization needs
an observed second use case or a demonstrated first-extension requirement.

## Upstream rationale

Dioxus documents sharing components as libraries and composing them in an app:
https://dioxuslabs.com/learn/0.7/tutorial/component/ . Bevy provides a comparable
build-time registration model: https://bevy.org/learn/quick-start/getting-started/plugins/ .
Zellij's lifecycle separates event updates from rendering within host-owned space:
https://zellij.dev/documentation/plugin-lifecycle.html . These inform the design;
none supplies a drop-in Styrene extension protocol. Dioxus guide examples must be
checked against the application's selected version during the spike.
