# Mobile Integration Corpus

This directory is a versioned UI-side copy of the backend-owned mobile
integration contract.

- Source repository: `https://github.com/styrene-lab/styrene-rs.git`
- Source revision: `0bcf5843208a9a2578836e26b4ac4e23a0f7b4e7`
- Source path: `tests/fixtures/mobile-integration-v1/corpus.json`

`styrene-rs` remains authoritative. Refresh this copy from an immutable backend
revision rather than changing protocol or destination-validation semantics here.
The `mobile.identity.copy-public-destination` case governs the copy and QR UI
flow implemented by `styrene-ui`.
