# Design

Keep a separately owned replaceable Dioxus task for page reads. Cancel its local
future on replacement, stop, close or session shutdown; preserve backend-operation
semantics and do not claim remote cancellation. Wrap reads in a local Tokio
deadline, and handle navigation errors through visible PageView state rather than
logging only. The existing broker request budget stays independent.

Preserve redaction of provider messages. Add a typed local-wait outcome and derive
safe failure text from typed stage metadata. Store at most 128 recent node outcomes
in ContentStore; clear them with session replacement. Search does not trigger probes.
Page-host labels never infer availability from status/announce colors.

Verify pure projections, timeout/drop behavior, stale-generation and recorded
outcomes, then desktop tests/corpus/build and captured Live/fixture views. Native
input automation remains dependent on host Accessibility permission.
