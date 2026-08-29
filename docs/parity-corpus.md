# Parity Corpus Contract

`styrene-rs` is authoritative for protocol behavior, fixture schemas, fixture
bytes, and parity claims. This repository owns presentation fixtures and
packaged-target replay evidence only.

The current integration revision is
`f15ce939887655ecb9ca4a7cdfa9e7378496dea5` from
`https://github.com/styrene-lab/styrene-rs.git`.

| Corpus | Backend path | UI status | SHA-256 |
|---|---|---|---|
| Mobile minimum v1 (`styrene-mobile-minimum-v1`) | `tests/fixtures/mobile-minimum-v1/states.json` | Mirrored semantically under `tests/fixtures/mobile-minimum-v1` | Canonical JSON `64dadb685eddad5dd80d954708eeb894a1784d8dbd5c025122a093ac6110b00c` |
| Mobile integration v1 (`styrene-mobile-integration-v1`) | `tests/fixtures/mobile-integration-v1/corpus.json` | Authoritative acceptance inventory; rows are referenced by ID in packaged evidence | File `d8ea971e98675d8a7344a7b468b55cbbf4a0b108d077ae2531f0b481ee15f2de` |
| Mobile application parity v1 | Proposed path `tests/fixtures/mobile-application-parity-v1/corpus.json` | Not admitted because no tracked corpus exists at the integration revision | N/A |
| RNS fixture index v2 | Planned path `tests/interop/fixtures/rns/index-v2.json` | Not consumed by UI until the tracked backend index and validator land | N/A |

The planned RNS v2 authority identifies Reticulum 1.5.1 revision
`149e4151095adf098b8f53eab0c03b37169e8559`. That plan and local failing tests
do not establish executable parity evidence. The currently tracked protocol
manifest remains schema v1 with Reticulum 1.4.2 revision
`b48b96e61676504e0a4e527b33b9a0b4495c6872`.

Component fixtures prove rendering and reducer behavior only. Packaged iOS and
Android runs must identify the backend revision, UI revision, artifact hash,
platform, OS, applicable mobile-integration row, and correlation. Protocol
claims additionally require the backend's pinned interoperability gate.
