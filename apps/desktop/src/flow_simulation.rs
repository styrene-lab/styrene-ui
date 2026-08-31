use serde::Deserialize;

use crate::backend::{self, BackendSession, ConnectionGeneration, FixtureId, RuntimeProfile};
use crate::state::{PageView, PeerRole};
use crate::stores::DomainStores;

const CORPUS: &str = include_str!("../../../tests/desktop/use-cases.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UseCaseCorpus {
    schema_version: u16,
    use_cases: Vec<UseCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UseCase {
    id: String,
    fixture: String,
    route: String,
    steps: Vec<String>,
    expected_visible_state: Vec<String>,
}

struct Simulation {
    backend: std::sync::Arc<dyn BackendSession>,
    generation: ConnectionGeneration,
    stores: DomainStores,
}

async fn open_fixture(fixture: FixtureId) -> Result<Simulation, String> {
    let generation = ConnectionGeneration::next();
    let mut opened = backend::open_session(RuntimeProfile::fixture(fixture), generation).await?;
    let mut stores = DomainStores::default();
    stores.begin_session("Fixture", generation);
    while let Some(event) = opened.events.recv().await {
        stores.apply_daemon_event(generation, event);
    }
    stores.set_interfaces(generation, opened.backend.interface_stats().await?);
    stores.set_links(generation, opened.backend.links().await?);
    Ok(Simulation { backend: opened.backend, generation, stores })
}

fn fixture_id(value: &str) -> Result<FixtureId, String> {
    match value {
        "empty" => Ok(FixtureId::Empty),
        "healthy" => Ok(FixtureId::Healthy),
        "degraded" => Ok(FixtureId::Degraded),
        "active-scenario" => Ok(FixtureId::ActiveScenario),
        "error" => Ok(FixtureId::Error),
        _ => Err(format!("unsupported desktop flow fixture '{value}'")),
    }
}

async fn run_use_case(use_case: &UseCase) -> Result<(), String> {
    if use_case.route.is_empty()
        || use_case.steps.is_empty()
        || use_case.expected_visible_state.is_empty()
    {
        return Err("use case must declare a route, steps, and visible outcomes".into());
    }

    let fixture = fixture_id(&use_case.fixture)?;
    if fixture == FixtureId::Error {
        let generation = ConnectionGeneration::next();
        let error = backend::open_session(RuntimeProfile::fixture(fixture), generation)
            .await
            .err()
            .ok_or_else(|| "error fixture opened unexpectedly".to_string())?;
        if !error.contains("Fixture error state") {
            return Err(format!("unexpected error fixture result: {error}"));
        }
        return Ok(());
    }

    let mut simulation = open_fixture(fixture).await?;
    match use_case.id.as_str() {
        "empty-session" => {
            if !simulation.stores.runtime.connected
                || !simulation.stores.network.peers.is_empty()
                || !simulation.backend.conversations().await?.is_empty()
            {
                return Err("empty session did not retain its connected empty state".into());
            }
        }
        "inspect-network" => {
            if simulation.stores.network.peers.len() != 3
                || simulation.stores.network.paths.len() != 3
                || simulation.stores.network.interfaces.len() != 1
            {
                return Err("healthy network inventory is incomplete".into());
            }
            if simulation.stores.mutation_availability("network.announce").is_ok() {
                return Err("fixture session unexpectedly authorized a network mutation".into());
            }
        }
        "send-message" => {
            let conversation = simulation
                .backend
                .conversations()
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| "healthy fixture has no conversation".to_string())?;
            let mut request = styrene_ipc::types::SendChatRequest::default();
            request.peer_hash = conversation.peer_hash.clone();
            request.content = "Desktop flow simulation".into();
            request.delivery_method = Some("direct".into());
            let outcome = simulation.backend.send_chat_outcome(request).await?;
            if !simulation.stores.apply_send_outcome(
                simulation.generation,
                conversation.peer_hash,
                "Desktop flow simulation".into(),
                outcome,
            ) || simulation
                .stores
                .messages
                .messages
                .iter()
                .all(|message| message.content != "Desktop flow simulation")
            {
                return Err("accepted message did not reach visible message state".into());
            }
        }
        "browse-content" => {
            let peer = simulation
                .stores
                .network
                .peers
                .iter()
                .find(|peer| peer.node_role == PeerRole::PageHost)
                .ok_or_else(|| "healthy fixture has no page host".to_string())?;
            let response = simulation.backend.browse_page(&peer.hash, "/page/index.mu").await?;
            simulation.stores.set_page(simulation.generation, PageView::from_daemon(response.page));
            let rendered = simulation
                .stores
                .content
                .page
                .as_ref()
                .and_then(|page| page.authoritative.as_ref())
                .map(|page| page.rendered_text.as_str());
            if rendered != Some("# Fixture Page\n\nDeterministic content from Fixture mode.") {
                return Err("fixture page did not reach visible content state".into());
            }
        }
        "inspect-degraded-session" => {
            if simulation.stores.network.status.transport_active
                || simulation.stores.network.interfaces.iter().all(|item| item.status != "degraded")
            {
                return Err("degraded fixture appeared healthy".into());
            }
        }
        "inspect-propagation" => {
            let snapshot = simulation.backend.propagation_snapshot(None).await?;
            simulation.stores.set_propagation_snapshot(simulation.generation, snapshot, false);
            let snapshot = simulation
                .stores
                .propagation
                .snapshot
                .as_ref()
                .ok_or_else(|| "active fixture has no propagation snapshot".to_string())?;
            if !snapshot.enabled || snapshot.queue_count != 1 || snapshot.queue.len() != 1 {
                return Err("active propagation state is incomplete".into());
            }
        }
        "session-error" => return Err("error fixture bypassed its failure assertion".into()),
        id => return Err(format!("desktop flow '{id}' has no simulation")),
    }
    simulation.backend.shutdown().await;
    Ok(())
}

#[test]
fn desktop_flow_corpus_is_versioned_and_unique() {
    let corpus: UseCaseCorpus = serde_json::from_str(CORPUS).expect("valid desktop flow corpus");
    assert_eq!(corpus.schema_version, 1);
    let mut ids = std::collections::HashSet::new();
    for use_case in corpus.use_cases {
        assert!(ids.insert(use_case.id), "duplicate desktop flow ID");
    }
    assert_eq!(ids.len(), 7, "standard desktop flow count changed");
}

#[tokio::test]
async fn desktop_flow_corpus_executes_standard_use_cases() {
    let corpus: UseCaseCorpus = serde_json::from_str(CORPUS).expect("valid desktop flow corpus");
    for use_case in corpus.use_cases {
        if let Err(error) = run_use_case(&use_case).await {
            panic!("desktop flow '{}' failed: {error}", use_case.id);
        }
    }
}
