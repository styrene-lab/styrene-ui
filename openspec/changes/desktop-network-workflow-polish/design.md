# Desktop Network Workflow Polish Design

## Capability Projection

The desktop must derive aggregate availability from the displayed actions, not
only from control-plane negotiation. Each action keeps its typed capability for
authorization. A presentation adapter maps denial classes to operator-safe text
and retains the technical identifier only in diagnostics.

The aggregate state has three values: available, partially available, and
read-only. Input validation is separate from authorization. Missing input must
not make an authorized action appear unauthorized.

## Workflow State

Destination discovery, active-link control, and native requests use independent
form state. Selecting an observed link or peer can populate the relevant form,
but one workflow must not silently reuse another workflow's value.

## Observation Relationships

Operations, native requests, and resource transfers remain authoritative IPC
records. The UI may display a relationship only when an authoritative
identifier establishes it. Missing relationships remain explicit.

## Rollout

The surface polish can land before this change. The deeper change must preserve
the current safety confirmation and generation checks. Fixture tests are the
acceptance boundary before runtime smoke testing.

## Reassessment

The deterministic capability, form-state, correlation, generation, and flow
simulation coverage is present on `main`. The flow simulation remains ignored by
default and is not packaged UI evidence.

Three gates remain open. Native keyboard order, labels, disabled guidance, and
destructive confirmation need direct tests. Healthy, read-only, partially
available, and active-operation fixtures need retained captures. Live-failure and
Embedded smoke checks must run without changing daemon fallback behavior. Existing
deterministic simulations do not close these native and runtime gates.
