use std::collections::HashSet;

use serde::Deserialize;

const CORPUS: &str = include_str!("../../../tests/fixtures/mobile-qr-ingress-v1/corpus.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u8,
    #[serde(rename = "corpus")]
    id: String,
    selected_architecture: String,
    decoder: Decoder,
    limits: Limits,
    privacy: Privacy,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decoder {
    #[serde(rename = "crate")]
    crate_name: String,
    version: String,
    image_crate_version: String,
    image_formats: Vec<String>,
    image_default_features: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Limits {
    encoded_image_bytes: usize,
    image_width: u32,
    image_height: u32,
    decoded_pixels: u64,
    candidate_payload_bytes: usize,
    decoded_symbols: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Privacy {
    retain_encoded_image: bool,
    retain_grayscale_frame: bool,
    log_decoded_payload: bool,
    fixture_images: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    layer: String,
    owner_host: String,
    input: String,
    expected: String,
    first_test: String,
}

#[test]
fn qr_ingress_tdd_corpus_is_bounded_private_and_complete() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("QR corpus must deserialize strictly");

    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.id, "styrene-mobile-qr-ingress-v1");
    assert_eq!(corpus.selected_architecture, "system_image_capture_pure_rust_quircs");
    assert_eq!(corpus.decoder.crate_name, "quircs");
    assert_eq!(corpus.decoder.version, "0.10.3");
    assert_eq!(corpus.decoder.image_crate_version, "0.25.10");
    assert_eq!(corpus.decoder.image_formats, ["jpeg", "png"]);
    assert!(!corpus.decoder.image_default_features);
    assert_eq!(corpus.limits.encoded_image_bytes, 8 * 1024 * 1024);
    assert_eq!(corpus.limits.image_width, 4096);
    assert_eq!(corpus.limits.image_height, 4096);
    assert_eq!(corpus.limits.decoded_pixels, 4096 * 4096);
    assert_eq!(
        corpus.limits.candidate_payload_bytes,
        styrene_ui_platform::MAX_CANDIDATE_PAYLOAD_BYTES
    );
    assert_eq!(corpus.limits.decoded_symbols, 1);
    assert!(!corpus.privacy.retain_encoded_image);
    assert!(!corpus.privacy.retain_grayscale_frame);
    assert!(!corpus.privacy.log_decoded_payload);
    assert_eq!(corpus.privacy.fixture_images, "generated_in_memory");

    let required = [
        "qr.decode.jpeg.canonical",
        "qr.decode.png.canonical",
        "qr.decode.no-code",
        "qr.decode.ambiguous",
        "qr.decode.malformed-image",
        "qr.decode.unsupported-format",
        "qr.decode.encoded-oversized",
        "qr.decode.pixel-oversized",
        "qr.decode.payload-oversized",
        "qr.capture.cancelled",
        "qr.capture.denied",
        "qr.capture.stale-generation",
        "qr.compose.backend-validation",
        "qr.privacy.payload-free-failure",
    ];
    let ids = corpus.cases.iter().map(|case| case.id.as_str()).collect::<HashSet<_>>();
    assert_eq!(ids.len(), corpus.cases.len(), "QR case IDs must be unique");
    assert_eq!(ids, required.into_iter().collect());
    for case in &corpus.cases {
        assert!(matches!(case.layer.as_str(), "decoder" | "platform" | "composition" | "privacy"));
        assert!(matches!(case.owner_host.as_str(), "nucleus" | "cross-repo"));
        assert!(!case.input.trim().is_empty());
        assert!(!case.expected.trim().is_empty());
        assert!(!case.first_test.trim().is_empty());
        assert!(!case.input.contains("destination_hash="));
    }
}
