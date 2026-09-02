# Parity Corpus Contract

`styrene-rs` is authoritative for protocol behavior, fixture schemas, fixture
bytes, and parity claims. This repository owns presentation fixtures and
packaged-target replay evidence only.

The current integration baseline revision is
`60a6ee3f02bb6e77e002c5e500a1e109243f8724` from
`https://github.com/styrene-lab/styrene-rs.git`.

| Corpus | Backend path | UI status | SHA-256 |
|---|---|---|---|
| Mobile minimum v1 (`styrene-mobile-minimum-v1`) | `tests/fixtures/mobile-minimum-v1/states.json` | Mirrored semantically under `tests/fixtures/mobile-minimum-v1` | Canonical JSON `64dadb685eddad5dd80d954708eeb894a1784d8dbd5c025122a093ac6110b00c` |
| Mobile integration v1 (`styrene-mobile-integration-v1`) | `tests/fixtures/mobile-integration-v1/corpus.json` | 56-case authoritative acceptance inventory; referenced by ID, not mirrored | File `391932fe38c4841826d7130549554f7f5e89730099ac75f4791e9881abe6e59c` |
| Mobile application parity v1 (`styrene-mobile-application-parity-v1`) | `tests/fixtures/mobile-application-parity-v1/corpus.json` | Exact 11-row working copy with uncommitted Skywave build 9 candidate; not release or packaged evidence | File `2e6ea065cc414cb4d5e35669eb690736f12561dd837dd139c56ee920938b81cd` |
| Mobile destination convergence v1 (`styrene-mobile-destination-convergence-v1`) | `tests/fixtures/mobile-destination-convergence-v1/corpus.json` | Exact copy at backend revision `726ef4f4c65725bcf24449e4b18387e6d322f1fa`; proves frontend dispatch and embedded-session convergence only | File `533845aebb0e7b04b39d5957fa94856e113388b66d73903d919cc5a0ef37165f` |
| RNS fixture index v2 | `tests/interop/fixtures/rns/index-v2.json` | Tracked backend protocol authority; Not consumed by UI | File `67a7573fdf6433ea66717c5cfec6deb11173a80fc5addacb91003f3851cf3fea` |

The tracked RNS v2 index identifies Reticulum 1.5.1 revision
`149e4151095adf098b8f53eab0c03b37169e8559` and preserves Reticulum 1.4.2
revision `b48b96e61676504e0a4e527b33b9a0b4495c6872` for legacy vectors. These
backend authorities do not establish packaged UI or external-application
evidence.

Component fixtures prove rendering and reducer behavior only. Packaged iOS and
Android runs must identify the backend revision, UI revision, artifact hash,
platform, OS, applicable mobile-integration row, and correlation. Protocol
claims additionally require the backend's pinned interoperability gate.
