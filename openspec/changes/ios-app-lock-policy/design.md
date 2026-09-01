# iOS App Access Lock Design

## Reassessment

UI PR #12 merged the initial implementation at `6a1143665ff2afbc3da076d6b1c3eb326f3fe527`.
Hosted checks passed, and the PR retained physical observations for setup, one
default cold launch, and one same-boot once-per-boot relaunch. Those observations
do not cover the complete policy matrix.

The current decision logic reads global `NSUserDefaults` values and derives a boot
marker from wall-clock time minus system uptime. The only shared component test
checks labels and custody-separation text. It does not prove persistence, setup
marker timing, startup ordering, reboot behavior, negative authentication outcomes,
or exactly-once prompting within one process.

## Decision Boundary

Move policy evaluation behind a pure Rust decision function. Inputs include the
policy, setup state, current launch identity, current boot identity, and recorded
launch or boot satisfaction. The function returns whether authentication is
required. Apple adapters remain responsible for obtaining launch and boot identity,
persisting state, and invoking LocalAuthentication.

The default and every unrecognized persisted value resolve to `EveryLaunch`.
State is recorded only after the event it represents succeeds. Authentication
satisfaction follows successful device-owner authentication. Setup completion
follows successful preparation of a usable backend session.

## Startup Ordering

When the decision requires authentication, the owner must authenticate before it
constructs the backend runtime or opens private session state. Cancelled,
unavailable, and failed outcomes remain closed. An explicit retry can request
authentication again.

Authentication success followed by backend failure keeps satisfaction according
to the selected launch or boot policy. Internal backend retries in the same process
must not create a second `EveryLaunch` prompt.

## Custody Boundary

App Lock controls entry to the application session. Keychain custody controls
access to identity material. Neither event satisfies, mutates, or proves the other.
App Lock state contains no identity secret or custody result.

## Evidence Boundary

Pure tests are authoritative for the decision matrix. Adapter tests are
authoritative for persistence. Component tests are authoritative for presentation.
Physical observations are required for actual LocalAuthentication prompt counts
and post-reboot behavior. No one evidence class substitutes for another.
