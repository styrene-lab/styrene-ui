# Desktop Network Workflow Polish Tasks

## 1. Capability Presentation
<!-- specs: desktop-network-workflows -->

- [ ] Inventory displayed actions and their negotiated capability states
- [ ] Define the available, partially available, and read-only projection
- [ ] Map typed denial classes to operator-safe labels without exposing capability identifiers
- [ ] Add fixture and component tests for aggregate and per-action availability

## 2. Workflow Forms
<!-- specs: desktop-network-workflows -->

- [ ] Separate destination, link-control, and native-request form state
- [ ] Preserve required-input validation independently from authorization
- [ ] Add explicit peer and link selection handoffs where authoritative records exist
- [ ] Test keyboard order, labels, disabled guidance, and destructive confirmation

## 3. Observation Lifecycle
<!-- specs: desktop-network-workflows -->

- [ ] Inventory authoritative correlation fields for operations, requests, and resources
- [ ] Display known lifecycle relationships and explicit unknown relationships
- [ ] Consolidate empty, active, terminal, and cancellable observation states
- [ ] Verify generation changes cannot retain stale workflow or observation context

## 4. Runtime Verification
<!-- specs: desktop-network-workflows -->

- [ ] Run formatting, desktop tests, and warning-denied Clippy
- [ ] Capture healthy, read-only, partially available, and active-operation fixtures
- [ ] Run Live-failure and Embedded smoke checks without changing daemon fallback behavior
