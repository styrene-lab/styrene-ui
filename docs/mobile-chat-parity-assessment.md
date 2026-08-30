# Mobile Chat Parity Assessment

Assessment date: 2026-08-30

UI assessment base revision: `6c6ace7652644c6bf6dc27922029a4cf311eb8bd`

Backend integration revision: `899da81302c5f4e92f60a2fdaf396c26e813ba76`

## Scope

This assessment compares the shared Dioxus mobile application with the admitted
mobile application corpus and current backend contracts. It does not create a
release claim. `styrene-rs` remains authoritative for protocol behavior,
runtime state, fixture contracts, and parity decisions.

The evidence classes remain separate:

- Application observations can define a workflow floor after provenance-locked execution.
- Backend and protocol tests prove only their executed runtime or wire behavior.
- Shared fixtures prove reducer, selector, and rendered-document behavior.
- Packaged runs prove only the journeys executed on the recorded target.
- Physical bearer and RF runs require their own hardware and correlation records.

## Governing Result

Formal parity with an external mobile chat application remains `unevidenced`.
The backend has admitted 11 P0 parity rows, but every row has no floor evidence
and retains `unevidenced` status. Static Sideband and Reticulum MeshChat
inspection remains candidate evidence only.

The current evidence inventory is:

| Corpus | Size | Current use |
|---|---:|---|
| Mobile application parity v1 | 11 rows | Workflow requirements and conservative parity decisions |
| Mobile integration v1 | 56 cases | Backend-owned acceptance inventory |
| Mobile minimum v1 | 8 fixtures | Shared reducer and rendered-state contract |

The UI copy of the application corpus is byte-identical to backend revision
`899da81302c5f4e92f60a2fdaf396c26e813ba76`. Copying the corpus does not add
application execution evidence or complete a parity row.

## Reference Applications

- Sideband `2.1.0` build `20251128` has a pinned Android APK hash. Static
  inspection covers all 11 journeys, but no retained Android execution exists.
- Reticulum MeshChat `2.4.0` has pinned source provenance. Its Android procedure
  uses Termux, Python, and a browser rather than a native APK.
- Skywave `1.0` build `5` has an incomplete retained summary. It cannot establish
  a workflow floor without exact platform, OS, artifact, and primary evidence.
- NomadNet `1.2.8` remains outside the mobile messaging minimum.
- Python RNS and LXMF revisions are protocol authorities, not interaction references.
- Meshtastic and MeshCore remain interaction-only references.

Sideband and Reticulum MeshChat cannot replace packaged Styrene evidence. Their
displayed delivery states also cannot establish authenticated receipt semantics.

## Backend Readiness

The backend P0 contract is 34 of 35 tasks complete. The remaining task requires
physical iOS and Android custody handoffs. The broader mobile minimum remains 48
of 73 tasks complete and retains separate packaged, RF, and application gates.

Backend revision `899da81302c5f4e92f60a2fdaf396c26e813ba76` provides these UI-ready contracts:

- Fail-closed Keychain and Android Keystore selection with secret-free custody status.
- A distinct offline-ready runtime state and generation-scoped session truth.
- Durable empty-conversation creation from a canonical destination.
- Durable contacts, aliases, drafts, unread state, messages, and attempt history.
- Complete message lifecycle, receipt, propagation, route, bearer, and failure evidence.
- Durable-before-ack propagation polling with bounded Unicode-safe previews.
- Bounded mobile diagnostics and deterministic redacted export.
- Persistent correlated echo behavior for controlled end-to-end testing.

These contracts resolve the backend defects listed in the previous assessment.
They do not prove that the UI projects each field or that a packaged device
executes the corresponding journey.

## Journey Assessment

These statuses describe current implementation and evidence. They are not
formal application-parity outcomes.

| Journey | Current assessment | Main gap |
|---|---|---|
| Shared iOS and Android shell | Strong shared-source evidence | Android packaged replay is absent |
| Identity display and custody | Shared projection implemented | Physical handoffs remain open |
| TCP setup and reconnect | Strong partial | Packaged endpoint-failure and reconnect journeys are absent |
| Canonical peer discovery | Backend ready, UI incomplete | The live UI does not start an empty conversation from discovery |
| Conversations and drafts | Strong partial | Packaged failure, restart, and upgrade journeys are absent |
| Direct bidirectional chat | Backend executable | No packaged cross-platform round trip exists |
| Unread and mark-read | Partial | Packaged semantic evidence is absent |
| Delivery truth | Backend ready, UI incomplete | Live projection discards detailed attempt and delivery evidence |
| Retry | Backend executable | Packaged retry and duplicate-prevention evidence is absent |
| Standard propagation client | Backend executable | The composer is not gated by current propagation readiness |
| Restart restoration | Backend executable | Physical process-death and upgrade evidence is absent |
| Degraded state | Backend ready, UI incomplete | Several live phases still collapse into `Reconnecting` |
| Settings and diagnostics | Strong partial | Capabilities and diagnostic export are not surfaced |

## UI Gaps

The following product gaps block packaged acceptance:

1. `project_message` does not preserve complete attempt, receipt, propagation,
   route, bearer, and backend failure evidence.
2. The live session projection collapses distinct backend phases into
   `Reconnecting`.
3. A discovered canonical peer cannot start a durable empty conversation.
4. Propagated composition is available without a selected ready propagation node.
5. The action channel can reject work without a surfaced typed result.
6. No tracked Android packaged automation harness exists.
7. No packaged live send, reply, unread, retry, reconnect, propagation, process
   death, or upgrade journey exists on both platforms.

These are UI and packaged-evidence gaps. They must not be restated as missing
backend contracts.

## Current Evidence

The eight mobile-minimum fixtures cover runtime, messaging, propagation, and
generation states. Shared Rust tests render these states for iOS and Android
target classes. The exact fixtures include live empty state, TCP reconnecting,
canonical discovery, queued direct delivery, and propagation upload without
delivery. They also include propagation sync completion, stale generation
rejection, and a recoverable failure.

The packaged iOS suite covers fixture navigation, primary target size,
background and foreground preservation, landscape navigation, and limited
Dynamic Type reflow. It does not prove VoiceOver, live messaging, protocol
interoperability, or delivery.

Physical iOS BLE observations cover approval, reconnect, interruption, Forget,
and reapproval for the recorded RNode. Physical Android USB observations cover
permission, configuration readback, detach, reconnect, and host acceptance of
an outbound KISS frame. Neither set proves RF transmission, remote reception,
bidirectional message correlation, or general RNode support.

## Claim Boundary

The current evidence supports these statements:

- One Rust and Dioxus presentation implements the eight-state mobile minimum at
  reducer and rendered-document level for both target classes.
- The backend supplies the P0 custody, lifecycle, messaging, propagation,
  attempt-evidence, diagnostics, and correlated-echo contracts.
- Narrow iOS packaged layout and navigation behavior has executed evidence.
- Narrow iOS BLE and Android USB host behavior has physical evidence for the
  recorded hardware and scenarios.

The current evidence does not support these claims:

- External mobile chat application parity.
- Released cross-platform mobile chat parity.
- General BLE, USB, RNode, or RF messaging support.
- Packaged Android chat behavior.
- VoiceOver, TalkBack, or WCAG 2.2 Level AA conformance.
- Notification delivery or guaranteed background execution.
- Attachments, Paper delivery, NomadNet parity, or propagation hosting.

## Follow-Up Order

1. Execute physical custody handoffs against the secret-free custody projection.
2. Preserve complete live lifecycle and delivery evidence in shared UI state.
3. Add discovery-to-conversation and propagation-readiness actions.
4. Surface typed action-channel rejection.
5. Add tracked Android packaged automation.
6. Run packaged TCP, messaging, propagation, process-death, and upgrade journeys.
7. Run scoped Android USB and RF correlation with retained evidence.
8. Execute the pinned Sideband APK and classify all parity rows conservatively.
