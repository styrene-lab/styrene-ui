# Mobile Destination Convergence Corpus

`corpus.json` is copied byte-for-byte from `styrene-rs` at
`tests/fixtures/mobile-destination-convergence-v1/corpus.json`.

Source fixture revision: `21dec9ffd4a97e9647c951b52e3b4dfdd84e91f2`

Current mobile backend dependency revision: `23beb83dbed95165347debdaabb1a672febfdc92`

File SHA-256: `533845aebb0e7b04b39d5957fa94856e113388b66d73903d919cc5a0ef37165f`

The backend owns the candidate set, the canonical destination, and the
validation rule. This repository proves two frontend facts. Discovered, manual,
pasted, and scanned candidates reach one `StartConversation` action unchanged.
The embedded session then forwards that action to the single backend operation.

Run both halves before claiming convergence:

```sh
cargo test -p styrene-ui-state --test destination_convergence
cargo test -p styrene-mobile --lib destination_convergence
```

Component and embedded-session tests are not packaged or physical-device
evidence.
