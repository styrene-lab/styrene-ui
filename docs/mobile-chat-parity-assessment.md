# Mobile Chat Parity Assessment

Assessment date: 2026-08-29

UI revision: `f4944c70e61653aad3489b4d8d0c34e1df664f4f`

Backend integration revision: `7dbe68e0e7a2e5e657e4c6c55b304a6a009ab992`

## Scope

This document assesses the shared Dioxus mobile application against the
repository's established mobile chat evidence. It does not define new product
requirements. `styrene-rs` remains authoritative for protocol behavior,
fixtures, and parity claims.

The assessment keeps these evidence classes separate:

- External application observations define an operator workflow floor only
  after admission to the application-parity corpus.
- Backend and protocol tests establish only their executed runtime or wire
  behavior.
- Mobile-minimum fixtures establish deterministic state and rendered-document
  behavior.
- Packaged simulator, emulator, and physical-device runs establish only the
  workflows that they execute.

## Governing Result

Formal parity with external mobile chat applications is **unevidenced**. The
specified `mobile-application-parity-v1` corpus has not been admitted, and its
proposed `corpus.json` does not exist. There are no valid external application
rows that can receive `matched` or `intentionally_different` status.

The current evidence consists of:

| Corpus | Size | Current use |
|---|---:|---|
| Mobile application parity v1 | 0 admitted rows | Formal external application parity is not assessable |
| Mobile integration v1 | 50 cases | Backend-owned acceptance inventory; not an external application parity ledger |
| Mobile minimum v1 | 8 fixtures | Shared reducer and rendered-state contract for iOS and Android target classes |

The mobile integration inventory records 35 P0 cases and 15 P1 cases. Its
tracked maturity values are 2 executable, 24 partial, and 24 blocked. These
values are an inventory snapshot, not a current release verdict. Several
capabilities listed as missing are now present in source but lack the required
packaged evidence.

## Reference Applications

The planned reference classifications remain conservative:

- Skywave `1.0` build `5` is observed for public TCP connectivity and canonical
  `lxmf.delivery` announces only. It does not establish message, receipt, or
  propagation-client parity.
- NomadNet `1.2.8` is pinned, but its bidirectional application gates remain
  incomplete and outside the mobile minimum.
- Sideband, Columba, and MeshChat remain unevidenced candidates.
- Meshtastic and MeshCore remain interaction-only references. They cannot
  establish RNS or LXMF behavior.
- Pinned Python RNS and LXMF implementations are protocol authorities, not
  mobile interaction references.

Skywave's local propagation hosting is not part of the Styrene mobile product.
If an application row is admitted, this difference must be recorded as an
intentional client-only product boundary. Styrene's third-party propagation-node
selection is a stronger workflow and is not supported by the current Skywave
observation.

## Journey Assessment

The statuses in this table are implementation assessments. They are not formal
application-parity statuses.

| Journey | Current assessment | Main gap |
|---|---|---|
| Shared iOS and Android shell | Strongly aligned in shared source | Android packaged replay is incomplete |
| Identity display and custody | Partial | No copy action, identity edit, custody status, or complete restart and upgrade evidence |
| TCP setup and reconnect | Strong partial | Packaged endpoint-failure and reconnect journeys are absent |
| Canonical peer discovery | Partial | A discovered peer cannot start a new canonical conversation |
| Existing conversations and drafts | Strong partial | Packaged send, failure, and restart journeys are absent |
| Direct bidirectional chat | Backend-only executable | No packaged iOS, Android, or cross-platform round trip |
| Unread and mark-read | Partial | Packaged semantic evidence is absent |
| Delivery truth | Partial | Live attempt, receipt, correlation, fallback, route, and bearer details are incomplete |
| Retry | Partial | Retry capability and duplicate-prevention evidence are incomplete in the packaged product |
| Standard propagation client | Strong partial | No packaged Brutus upload, retrieval, acknowledgement, and repeat-sync evidence |
| Restart restoration | Partial | No packaged process-death restoration of conversations, drafts, outbox, and correlation |
| Accessibility | Partial | Focus transitions, complete large-text reflow, VoiceOver, TalkBack, and WCAG accounting remain open |
| Settings and diagnostics | Mostly absent | More exposes identity and build information only |
| Attachments and pages | Intentionally outside the mobile minimum | Keep these workflows excluded unless scope changes |

## Material Implementation Gaps

The following gaps affect the mobile messaging minimum:

1. The UI cannot create the first conversation from a discovered delivery
   destination.
2. Live message projection is less complete than fixture projection. Transport,
   propagation, attempt, receipt, correlation, selected node, fallback, route,
   and bearer evidence are incomplete or not displayed.
3. The composer permits Propagated submission without a selected, ready
   propagation node. It relies on a downstream generic rejection.
4. Several backend session phases collapse into `Reconnecting`, so stopped,
   connecting, reconnecting, and degraded states are not distinct in the UI.
5. The mobile action channel can reject work without projecting a typed busy or
   unavailable result.
6. No packaged Android chat test suite exists.
7. No packaged live send, reply, unread, retry, reconnect, propagation, or
   process-restart journey exists on either platform.

## Backend Readiness

The backend assessment covers 22 capability areas at backend revision
`7dbe68e0e7a2e5e657e4c6c55b304a6a009ab992`:

- 10 areas are complete and directly projectable.
- 6 areas have substantial backend support but need a mobile adapter or a more
  accurate UI projection.
- 6 areas require a new backend contract or durable backend state.

These counts describe source readiness. They do not replace packaged or
interoperability evidence.

### Directly Projectable

| Capability | Backend state | UI work |
|---|---|---|
| Contacts and aliases | Durable contact CRUD and restart persistence exist through typed IPC and `MobileNode` | Add contact state, alias workflows, and display-name precedence |
| Drafts | Revisioned drafts and conditional clearing are durable | Preserve and render the authoritative revision and clear disposition |
| Send and outbox | Send persists before dispatch and returns a complete authoritative message | Project the returned message and terminal failure without replacing them with generic state |
| Retry | Retry retains one canonical message and records another attempt | Render retry dispositions and attempt history |
| Unread and mark-read | Unread state is durable; active-conversation handling marks new inbound messages read | Add packaged semantic assertions and avoid local unread approximations |
| Message lifecycle | Queued, sending, sent, delivered, failed, cancelled, expired, and rejected are typed | Preserve every live state and its terminal semantics |
| Delivery method and attempts | Requested and actual method, fallback, correlation, and attempts are present in `MessageInfo` | Extend `styrene-ui-state::Message` and `project_message` |
| Receipt and resource evidence | Packet receipt and resource completion evidence includes exact hashes, state, outcome, timestamps, and progress | Add truthful expandable delivery detail |
| Propagation message evidence | Durable propagation correlations include selected peer, transient and attempt IDs, relation, state, and timestamps | Distinguish upload from recipient delivery |
| Propagation selection and sync | Candidates, policy, readiness, persisted selection, progress, failure, and bounded sync are exposed by `MobileNode` | Gate Propagated composition on the projected readiness state |

Primary source anchors:

- `styrene-rs/crates/libs/styrene-ipc/src/types.rs:409-570` defines the
  authoritative lifecycle, message, attempt, receipt, and propagation evidence.
- `styrene-rs/crates/apps/styrened/src/mobile.rs:1808-2250` exposes typed send,
  retry, propagation, draft, and active-conversation operations.
- `styrene-rs/crates/apps/styrened/src/mobile.rs:2632-2707` exposes conversation,
  contact, and read operations.
- `styrene-rs/crates/apps/styrened/tests/mobile_node.rs:646-835` tests failed-send
  restoration, retry correlation, draft revision safety, unread restoration,
  and complete generation-scoped message events.
- `styrene-rs/crates/apps/styrened/tests/daemon_facade_contract.rs:237-269`
  tests requested-method fallback and attempt evidence; lines 757-818 test
  contact and conversation-state restart persistence.

The current live UI discards much of this evidence. In
`apps/mobile/src/session.rs:848-869`, `project_message` keeps method, lifecycle,
and correlation, but hard-codes transport and propagation evidence to `None`,
reduces delivery to pending or delivered, and replaces the backend failure with
a generic `terminal_message` failure. The backend does not need to be extended
before this projection is corrected.

### Adapter Or Projection Work

| Capability | Existing backend support | Remaining boundary work |
|---|---|---|
| Public identity and copy | Public identity and destination hashes are available through typed IPC | Add a focused mobile identity projection and host clipboard action |
| Identity edit | Typed edit and re-announce exist | Add a mobile wrapper and persist edited metadata across process death before claiming restoration |
| Session lifecycle | The mobile contract declares stopped, starting, connecting, connected, reconnecting, degraded, and failed | The backend snapshot currently emits only stopped, connecting, connected, and reconnecting; the UI then collapses all nonterminal transitional states into `Reconnecting` |
| Restart restoration | Identity, endpoint, messages, attempts, drafts, contacts, and unread state reopen from durable storage | Add forced-process-death, incomplete-operation, migration, and packaged host restoration evidence |
| Pagination | Stable cursor-based conversation and message pages exist in IPC and storage | Add direct paged `MobileNode` methods and UI paging state instead of using the generic daemon escape hatch |
| Settings and capabilities | Runtime capabilities, authorization, endpoint persistence, and durable conversation mute exist | Define a generation-scoped mobile settings snapshot and focused wrappers |

Relevant source anchors:

- `styrene-rs/crates/apps/styrened/src/mobile.rs:123-141` declares the lifecycle
  contract, while lines 1689-1748 derive the current session snapshot.
- `styrene-ui/apps/mobile/src/session.rs:895-904` currently collapses stopped,
  starting, connecting, reconnecting, and degraded into one UI state.
- `styrene-rs/crates/apps/styrened/src/daemon_facade.rs:448-495` implements
  identity query, edit, and announce.
- `styrene-rs/crates/apps/styrened/src/daemon_facade.rs:864-950` exposes stable
  conversation and message page boundaries.

### Backend Work Required

| Capability | Missing backend contract |
|---|---|
| First conversation from discovery | Conversations are grouped from persisted messages only. There is no durable create/open-empty-conversation operation or draft-only conversation projection. Discovery, arbitrary-destination send, and draft persistence already exist, so this can be a narrow messaging/storage addition. |
| Identity custody status | Keychain, Android Keystore, encrypted-file, and development plaintext backends exist, but no public DTO reports the selected backend, protection level, availability, fallback, or custody health. |
| Per-message bearer and path evidence | Current bearer state and general path queries exist separately, but `MessageInfo` does not correlate an interface, bearer, next hop, or path observation to a delivery attempt. |
| Durable action queue | Backend operations are direct async calls and do not expose a generation-scoped command queue, idempotency key, enqueue disposition, or queue state. A smaller UI-only fix can stop silently dropping actions and report busy state; durable replay requires backend design. |
| Notifications and background execution | Message polling and conversation mute exist, but OS authorization, scheduling, badge, notification-open routing, iOS background-task, and Android foreground-service contracts do not. |
| Mobile diagnostics and export | Internal diagnostics and bounded event streams exist, but there is no typed chronological mobile diagnostic projection, bounded redacted export, or share-ready artifact. |

The first-conversation gap is visible in
`styrene-rs/crates/apps/styrened/src/storage/messages.rs:4143-4197`, where the
conversation query groups only the `messages` table. The absent custody fields
are visible in `styrene-rs/crates/libs/styrene-ipc/src/types.rs:66-75`. The
message evidence boundary is visible in the same file at lines 474-522: it
contains delivery evidence but no correlated bearer or path fields.

## Implementation Split

The following UI work can start without waiting for new protocol or transport
behavior:

1. Extend shared message state and `project_message` to retain attempts,
   receipts, propagation correlations, fallback reasons, and backend failures.
2. Expand shared session state enough to preserve the four phases that the
   backend currently emits. Do not render stopped or connecting as reconnecting.
3. Pass propagation readiness into the composer and disable Propagated send when
   no selected node is ready.
4. Add contacts and aliases through the existing `MobileNode` methods.
5. Make action submission return a typed accepted, busy, or unavailable result
   instead of ignoring `try_send` failure.
6. Add packaged journeys for the already implemented draft, send, failure,
   retry, unread, reconnect, and propagation workflows.

The following work should include a small backend change before UI completion:

1. Add a durable empty-conversation or draft-only conversation projection, then
   add the discovery-to-conversation action.
2. Add focused paged mobile methods rather than binding mobile UI code to the
   full generic daemon trait.
3. Add durable identity metadata and a custody-status DTO before showing an
   authoritative identity security status.
4. Add per-attempt bearer/path correlation before claiming route or bearer
   evidence in message details.
5. Define notification/background and diagnostic-export contracts before adding
   those settings surfaces.

## Current Evidence

The eight mobile-minimum fixtures cover live empty state, TCP reconnecting,
canonical discovery, direct queued delivery, propagation upload without
delivery, propagation sync completion, stale generation rejection, and a
recoverable failure. Shared Rust tests render these states for both target
classes.

The packaged iOS suite covers fixture navigation, primary target size,
background and foreground preservation, landscape navigation, and limited
Dynamic Type reflow. This is accessibility-tree and frame evidence. It is not
VoiceOver, live messaging, protocol interoperability, or delivery evidence.

Physical iOS BLE observations cover approval, persisted reconnect, interruption,
Forget, and reapproval for the exercised device and RNode. Physical Android USB
observations cover permission, configuration readback, detach, reconnect, and
host acceptance of an outbound KISS frame. Neither evidence set proves RF
transmission, remote reception, bidirectional message correlation, or general
RNode support.

## Defensible Claim Boundary

The current source supports these limited statements:

- One shared Rust and Dioxus presentation implements the eight-state mobile
  minimum at reducer and rendered-document level for both target classes.
- Embedded TCP, discovery, messaging, and propagation plumbing is substantial,
  but packaged cross-platform acceptance remains incomplete.
- Narrow iOS packaged layout and navigation behavior has executed evidence.
- Narrow iOS BLE host and Android USB host behavior has physical evidence for
  the recorded hardware and scenarios.

The current evidence does not support these claims:

- External mobile chat application parity.
- Released cross-platform mobile chat parity.
- General BLE or RF messaging support.
- Packaged Android chat behavior.
- VoiceOver, TalkBack, or WCAG 2.2 Level AA conformance.
- Attachments, Paper delivery, NomadNet parity, propagation hosting, capacity,
  expiry administration, or background delivery.

## Follow-Up Order

1. Admit the backend-owned application corpus and P0 parity matrix.
2. Project complete live lifecycle and delivery evidence.
3. Add a typed start-conversation workflow from discovery.
4. Make composer availability depend on propagation readiness.
5. Add packaged send, reply, unread, retry, reconnect, propagation, and restart
   scenarios.
6. Add Android packaged parity and physical accessibility evidence.
