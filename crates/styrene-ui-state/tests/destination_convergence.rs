//! Frontend half of the destination convergence corpus.
//!
//! Discovered, manual, pasted, and scanned candidates must reach the backend
//! through one `StartConversation` action without frontend canonicalization or
//! optimistic state. Backend acceptance is proven separately in `styrene-rs`
//! and in the embedded session test of `styrene-mobile`.

use serde::Deserialize;
use styrene_ui_state::{
    DestinationEntryConstraint, LXMF_DESTINATION_INPUT_MAX_BYTES, MobileAction, MobileActionKind,
    bounded_destination_input, destination_entry_constraint, start_conversation_action,
};

const CORPUS: &str =
    include_str!("../../../tests/fixtures/mobile-destination-convergence-v1/corpus.json");
const GENERATION: u64 = 7;

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: u32,
    #[serde(rename = "corpus")]
    identity: String,
    canonical_peer_hash: String,
    ingress_paths: Vec<String>,
    converging_candidates: Vec<ConvergingCandidate>,
    rejected_candidates: Vec<RejectedCandidate>,
}

#[derive(Debug, Deserialize)]
struct ConvergingCandidate {
    id: String,
    ingress: String,
    raw: String,
    submitted: String,
}

#[derive(Debug, Deserialize)]
struct RejectedCandidate {
    id: String,
    raw: String,
    ui_dispatch: String,
}

fn corpus() -> Corpus {
    let corpus: Corpus =
        serde_json::from_str(CORPUS).expect("destination convergence corpus must deserialize");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.identity, "styrene-mobile-destination-convergence-v1");
    assert_eq!(corpus.ingress_paths, ["discovered", "manual", "pasted", "scanned"]);
    assert_eq!(corpus.canonical_peer_hash.len(), LXMF_DESTINATION_INPUT_MAX_BYTES);
    corpus
}

/// Mirrors the People directory action, which forwards the discovered hash as-is.
fn discovered_action(peer_hash: &str) -> MobileAction {
    MobileAction::new(
        GENERATION,
        MobileActionKind::StartConversation { peer_hash: peer_hash.into() },
    )
}

/// Mirrors New Message entry: pasted and scanned candidates land in the same
/// bounded destination field as manual typing before submission.
fn entered_action(raw: &str) -> Option<MobileAction> {
    start_conversation_action(GENERATION, &bounded_destination_input(raw))
}

#[test]
fn every_ingress_path_dispatches_one_start_conversation_action_without_canonicalizing() {
    let corpus = corpus();
    let mut seen_paths = Vec::new();

    for candidate in &corpus.converging_candidates {
        let action = if candidate.ingress == "discovered" {
            discovered_action(&candidate.raw)
        } else {
            entered_action(&candidate.raw)
                .unwrap_or_else(|| panic!("{} was blocked before dispatch", candidate.id))
        };
        assert_eq!(action.generation, GENERATION, "{}", candidate.id);
        assert_eq!(
            action.kind,
            MobileActionKind::StartConversation { peer_hash: candidate.submitted.clone() },
            "{} must reach the backend operation with its submitted value",
            candidate.id
        );
        assert_eq!(
            candidate.submitted.to_ascii_lowercase(),
            corpus.canonical_peer_hash,
            "{} must converge on the canonical destination at the backend",
            candidate.id
        );
        seen_paths.push(candidate.ingress.as_str());
    }

    seen_paths.sort_unstable();
    seen_paths.dedup();
    assert_eq!(seen_paths, ["discovered", "manual", "pasted", "scanned"]);
    assert!(
        corpus
            .converging_candidates
            .iter()
            .any(|candidate| candidate.submitted != corpus.canonical_peer_hash),
        "the corpus must prove the frontend forwards non-canonical casing for backend canonicalization"
    );
}

#[test]
fn rejected_candidates_follow_their_declared_frontend_dispatch() {
    let corpus = corpus();

    for candidate in &corpus.rejected_candidates {
        let bounded = bounded_destination_input(&candidate.raw);
        let constraint = destination_entry_constraint(&bounded);
        let action = entered_action(&candidate.raw);
        match candidate.ui_dispatch.as_str() {
            "forwarded" => {
                assert_eq!(constraint, DestinationEntryConstraint::Ready, "{}", candidate.id);
                assert_eq!(
                    action.map(|action| action.kind),
                    Some(MobileActionKind::StartConversation {
                        peer_hash: candidate.raw.trim().to_owned(),
                    }),
                    "{} must be forwarded for backend validation",
                    candidate.id
                );
            }
            "trimmed_before_dispatch" => {
                assert_ne!(candidate.raw, candidate.raw.trim(), "{}", candidate.id);
                assert_eq!(
                    action.map(|action| action.kind),
                    Some(MobileActionKind::StartConversation {
                        peer_hash: candidate.raw.trim().to_owned(),
                    }),
                    "{} must be trimmed before dispatch",
                    candidate.id
                );
            }
            "blocked_empty" => {
                assert_eq!(constraint, DestinationEntryConstraint::Empty, "{}", candidate.id);
                assert!(action.is_none(), "{} must be blocked", candidate.id);
            }
            "blocked_incomplete" => {
                assert_eq!(constraint, DestinationEntryConstraint::Incomplete, "{}", candidate.id);
                assert!(action.is_none(), "{} must be blocked", candidate.id);
            }
            "blocked_oversized" => {
                assert_eq!(constraint, DestinationEntryConstraint::Oversized, "{}", candidate.id);
                assert!(bounded.len() <= LXMF_DESTINATION_INPUT_MAX_BYTES + 1, "{}", candidate.id);
                assert!(action.is_none(), "{} must be blocked", candidate.id);
            }
            other => panic!("{} declares unknown frontend dispatch {other}", candidate.id),
        }
    }
}
