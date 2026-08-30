# Parity Corpus Contract

`styrene-rs` is authoritative for protocol behavior, fixture schemas, fixture
bytes, and parity claims. This repository owns presentation fixtures and
packaged-target replay evidence only.

The current integration revision is
`92eb6397deeca826d514855b8e4d82cac9068d1e` from
`https://github.com/styrene-lab/styrene-rs.git`.

| Corpus | Backend path | UI status | SHA-256 |
|---|---|---|---|
| Mobile minimum v1 (`styrene-mobile-minimum-v1`) | `tests/fixtures/mobile-minimum-v1/states.json` | Mirrored semantically under `tests/fixtures/mobile-minimum-v1` | Canonical JSON `64dadb685eddad5dd80d954708eeb894a1784d8dbd5c025122a093ac6110b00c` |
| Mobile integration v1 (`styrene-mobile-integration-v1`) | `tests/fixtures/mobile-integration-v1/corpus.json` | 56-case authoritative acceptance inventory; referenced by ID, not mirrored | File `391932fe38c4841826d7130549554f7f5e89730099ac75f4791e9881abe6e59c` |
| Mobile application parity v1 (`styrene-mobile-application-parity-v1`) | `tests/fixtures/mobile-application-parity-v1/corpus.json` | 11 admitted workflow and authority rows; not packaged evidence | File `a43b5e749202e25df1cd4bfbfc90e4d996b59d7e0375beff008ffb85617cbea4` |
| RNS fixture index v2 | `tests/interop/fixtures/rns/index-v2.json` | Tracked backend protocol authority; Not consumed by UI | File `57d479317d73595b6dad62afe17bfff0998f7a99e2150d28e3cdf2e3f6d46ec1` |

The tracked RNS v2 index identifies Reticulum 1.5.1 revision
`149e4151095adf098b8f53eab0c03b37169e8559` and preserves Reticulum 1.4.2
revision `b48b96e61676504e0a4e527b33b9a0b4495c6872` for legacy vectors. These
backend authorities do not establish packaged UI or external-application
evidence.

Component fixtures prove rendering and reducer behavior only. Packaged iOS and
Android runs must identify the backend revision, UI revision, artifact hash,
platform, OS, applicable mobile-integration row, and correlation. Protocol
claims additionally require the backend's pinned interoperability gate.
