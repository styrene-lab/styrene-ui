# Desktop Network Workflow Polish

## Intent

Make desktop network controls describe actual operator capabilities and keep
commands, required input, and authoritative observations distinct.

## Scope

This change defines aggregate capability status, operator-safe denial reasons,
workflow-specific form state, and relationships between operations, requests,
and resource observations. It includes desktop component and fixture coverage.

It excludes daemon authorization policy, IPC wire changes, protocol behavior,
and a redesign of the network graph.

## Success criteria

- Aggregate status never reports readiness when all displayed actions are unavailable.
- Operator-facing text does not expose internal capability identifiers.
- Each workflow owns only the input required for that workflow.
- Request and resource observations show their known lifecycle relationships without inference.
- Desktop fixtures cover ready, partially available, read-only, empty, and active states.
