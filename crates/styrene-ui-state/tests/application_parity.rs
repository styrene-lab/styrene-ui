use std::collections::HashSet;

use serde_json::Value;

const CORPUS: &str =
    include_str!("../../../tests/fixtures/mobile-application-parity-v1/corpus.json");

#[test]
fn application_parity_copy_contains_every_p0_journey_without_promoted_evidence() {
    let corpus: Value = serde_json::from_str(CORPUS).expect("application corpus must be JSON");
    assert_eq!(corpus["schema_version"], 1);
    assert_eq!(corpus["corpus"], "styrene-mobile-application-parity-v1");

    let rows = corpus["parity_rows"].as_array().expect("parity_rows must be an array");
    let actual = rows
        .iter()
        .map(|row| row["id"].as_str().expect("journey id must be a string"))
        .collect::<HashSet<_>>();
    let expected = HashSet::from([
        "mobile.journey.identity",
        "mobile.journey.tcp-setup",
        "mobile.journey.discovery",
        "mobile.journey.conversations",
        "mobile.journey.drafts",
        "mobile.journey.direct-send",
        "mobile.journey.receipts",
        "mobile.journey.retry",
        "mobile.journey.restart",
        "mobile.journey.propagation",
        "mobile.journey.degraded-state",
    ]);

    assert_eq!(rows.len(), expected.len());
    assert_eq!(actual, expected);
    assert!(rows.iter().all(|row| row["status"] == "unevidenced"));
    assert!(rows.iter().all(|row| row["floor_evidence_id"].is_null()));
}
