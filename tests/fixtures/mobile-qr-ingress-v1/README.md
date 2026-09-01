# Mobile QR Ingress Corpus

`corpus.json` defines test-first QR capture and decoding behavior for
`complete-mobile-product-workflows`.

The selected P0 architecture is a Dioxus file capture followed by bounded
pure-Rust decoding. Tests must generate images in memory. Do not add camera
frames or decoded scan payloads to this directory.

Android implementation and E87 evidence belong on `nucleus`. Apple build and
device evidence belong on `Chriss-MacBook-Pro`.

Run the corpus validator before implementation:

```sh
cargo test -p styrene-ui-platform --test qr_ingress_corpus
```

The corpus validator proves planning integrity. It does not prove decoder,
packaged application, camera, or device behavior.
