# Desktop Network Workflow Polish Tasks

## 1. Capability Presentation
<!-- specs: desktop-network-workflows -->

- [x] Inventory displayed actions and their negotiated capability states
- [x] Define the available, partially available, and read-only projection
- [x] Map typed denial classes to operator-safe labels without exposing capability identifiers
- [x] Add fixture and component tests for aggregate and per-action availability

## 2. Workflow Forms
<!-- specs: desktop-network-workflows -->

- [x] Separate destination, link-control, and native-request form state
- [x] Preserve required-input validation independently from authorization
- [x] Add explicit peer and link selection handoffs where authoritative records exist
- [ ] Test keyboard order, labels, disabled guidance, and destructive confirmation

## 3. Observation Lifecycle
<!-- specs: desktop-network-workflows -->

- [x] Inventory authoritative correlation fields for operations, requests, and resources
- [x] Display known lifecycle relationships and explicit unknown relationships
- [x] Consolidate empty, active, terminal, and cancellable observation states
- [x] Verify generation changes cannot retain stale workflow or observation context

## 4. Runtime Verification
<!-- specs: desktop-network-workflows -->

- [x] Add a versioned desktop use-case corpus and deterministic flow simulation
- [x] Run formatting, desktop tests, and warning-denied Clippy
- [ ] Capture healthy, read-only, partially available, and active-operation fixtures
- [ ] Run Live-failure and Embedded smoke checks without changing daemon fallback behavior
