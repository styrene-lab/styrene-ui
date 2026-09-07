## 1. Minimal composition and provider seam
<!-- specs: identity-ui-extension -->

- [ ] Agree the first public Identity summary/capability DTO and read-only request with the Identity owner; record immutable contract revision or explicit mock-only status.
- [ ] Add failing registration tests for optional absence, duplicate IDs and contract mismatch.
- [ ] Implement the smallest private registration module and optional Identity page adapter using the pinned Dioxus version; avoid a generic SDK until needed.
- [ ] Add page tests for unavailable/error/retry with no implicit unlock, identity mutation or CLI execution.

## 2. Lifecycle dogfood
<!-- specs: identity-ui-extension -->

- [ ] Add failing enable/disable/re-enable, repeated navigation and subscription-count tests.
- [ ] Implement activation/view scopes and generation filtering; test delayed results after session switch or disable.
- [ ] Persist the host-owned enabled preference; verify restart does not replay requests or persist secret input.
- [ ] Verify native desktop navigation, focus return and shell usability with the extension absent, unavailable and disabled.
- [ ] Record the smallest contract actually used and separate platform evidence; identify mobile suspend/resume work without claiming acceptance.

## 3. Gate before first lifecycle mutation
<!-- specs: identity-ui-extension -->

- [ ] Select one lifecycle mutation only after its Identity owner contract and CLI/service acceptance cases exist.
- [ ] Specify operation ID ownership, independent observation, capability revocation and cancellation behavior for that operation.
- [ ] Add failing tests for disable during pending work, including host observation and blocked-disable fallback; implement the applicable behavior.
- [ ] Record update/restart/data-retention implications exposed by that mutation; keep runtime installation and generalized extension slots deferred unless evidence requires them.
