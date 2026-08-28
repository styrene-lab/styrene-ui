//! Renderer-neutral presentation state for Styrene Dioxus applications.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobileMinimumCorpus {
    pub schema_version: u32,
    pub corpus: String,
    pub target_classes: Vec<TargetClass>,
    pub required_accessibility_ids: Vec<String>,
    pub fixtures: Vec<MobileFixture>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobileFixture {
    pub id: String,
    pub profile: Profile,
    pub generation: u64,
    pub session: Session,
    pub bearers: Vec<Bearer>,
    pub peers: Vec<Peer>,
    pub conversations: Vec<Conversation>,
    pub messages: Vec<Message>,
    pub propagation: Propagation,
    pub event: Option<GenerationEvent>,
    pub expected: ExpectedProjection,
}

impl MobileFixture {
    #[must_use]
    pub const fn accepts_generation(&self, generation: u64) -> bool {
        generation == self.generation
    }

    #[must_use]
    pub fn bearer(&self, kind: BearerKind) -> Option<&Bearer> {
        self.bearers.iter().find(|bearer| bearer.kind == kind)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyResult {
    Applied,
    IgnoredStale,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointUpdate {
    pub endpoint: String,
    pub generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MobileStore {
    snapshot: MobileFixture,
    local_announce_outcome: Option<LocalAnnounceOutcome>,
}

impl MobileStore {
    #[must_use]
    pub const fn new(snapshot: MobileFixture) -> Self {
        Self { snapshot, local_announce_outcome: None }
    }

    #[must_use]
    pub fn cold_restore(snapshot: MobileFixture, generation: u64) -> Self {
        let mut store = Self::new(snapshot);
        store.begin_reconnect(generation, "cold_restore");
        store
    }

    pub fn begin_reconnect(&mut self, generation: u64, reason: impl Into<String>) {
        if generation <= self.snapshot.generation {
            return;
        }

        self.snapshot.generation = generation;
        self.snapshot.session.phase = SessionPhase::Reconnecting;
        self.snapshot.session.failure = None;
        if let Some(tcp) =
            self.snapshot.bearers.iter_mut().find(|bearer| bearer.kind == BearerKind::Tcp)
        {
            tcp.state = BearerState::Reconnecting;
            tcp.reason = Some(reason.into());
        }
    }

    pub fn apply_snapshot(&mut self, generation: u64, snapshot: MobileFixture) -> ApplyResult {
        if generation != self.snapshot.generation || snapshot.generation != generation {
            return ApplyResult::IgnoredStale;
        }

        self.snapshot = snapshot;
        ApplyResult::Applied
    }

    pub fn apply_endpoint_update(&mut self, update: EndpointUpdate) -> ApplyResult {
        if update.generation <= self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }

        self.snapshot.session.endpoint = Some(update.endpoint);
        self.begin_reconnect(update.generation, "endpoint_changed");
        ApplyResult::Applied
    }

    pub fn apply_endpoint_failure(&mut self, failure: TypedFailure) {
        self.snapshot.session.failure = Some(failure);
    }

    pub fn apply_peer_snapshot(&mut self, snapshot: PeerSnapshot) -> ApplyResult {
        if snapshot.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }

        let mut peers = Vec::with_capacity(snapshot.peers.len());
        for peer in snapshot.peers {
            if let Some(existing) = peers
                .iter_mut()
                .find(|existing: &&mut Peer| existing.destination_hash == peer.destination_hash)
            {
                if peer.observed_at >= existing.observed_at {
                    *existing = peer;
                }
            } else {
                peers.push(peer);
            }
        }
        for peer in &mut peers {
            if let Some(current) = self
                .snapshot
                .peers
                .iter()
                .find(|current| current.destination_hash == peer.destination_hash)
            {
                if current.observed_at > peer.observed_at {
                    *peer = current.clone();
                }
            }
        }
        self.snapshot.peers = peers;
        ApplyResult::Applied
    }

    pub fn apply_peer_event(&mut self, event: PeerEvent) -> ApplyResult {
        if event.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }

        if let Some(existing) = self
            .snapshot
            .peers
            .iter_mut()
            .find(|peer| peer.destination_hash == event.peer.destination_hash)
        {
            if event.peer.observed_at < existing.observed_at {
                return ApplyResult::IgnoredStale;
            }
            *existing = event.peer;
        } else {
            self.snapshot.peers.push(event.peer);
        }
        ApplyResult::Applied
    }

    pub fn apply_local_announce_outcome(&mut self, outcome: LocalAnnounceOutcome) -> ApplyResult {
        if outcome.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }
        self.local_announce_outcome = Some(outcome);
        ApplyResult::Applied
    }

    #[must_use]
    pub const fn local_announce_outcome(&self) -> Option<&LocalAnnounceOutcome> {
        self.local_announce_outcome.as_ref()
    }

    #[must_use]
    pub const fn snapshot(&self) -> &MobileFixture {
        &self.snapshot
    }

    #[must_use]
    pub fn messaging_available(&self) -> bool {
        self.snapshot.bearers.iter().any(|bearer| bearer.state == BearerState::Connected)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub phase: SessionPhase,
    pub identity_hash: String,
    pub endpoint: Option<String>,
    pub failure: Option<TypedFailure>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bearer {
    pub kind: BearerKind,
    pub state: BearerState,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    pub destination_hash: String,
    pub aspect: String,
    pub display_name: Option<String>,
    pub observed_at: i64,
    pub age_secs: u64,
    pub source: PeerSource,
    pub announce_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerSnapshot {
    pub generation: u64,
    pub peers: Vec<Peer>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerEvent {
    pub generation: u64,
    pub peer: Peer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAnnounceOutcome {
    pub generation: u64,
    pub accepted_at: i64,
    pub local_dispatch_accepted: bool,
    pub remote_reception_confirmed: bool,
    pub failure: Option<TypedFailure>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerSource {
    CanonicalAnnounce,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Conversation {
    pub peer_hash: String,
    pub unread_count: u32,
    pub draft: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub id: String,
    pub peer_hash: String,
    pub content: String,
    pub requested_method: DeliveryMethod,
    pub actual_method: DeliveryMethod,
    pub persistence: PersistenceState,
    pub transport: TransportEvidence,
    pub propagation: PropagationEvidence,
    pub delivery: DeliveryEvidence,
    pub correlation_id: String,
    pub failure: Option<TypedFailure>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Propagation {
    pub selected_destination: Option<String>,
    pub ready: bool,
    pub sync_state: SyncState,
    pub new_messages: u32,
    pub failure: Option<TypedFailure>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationEvent {
    pub generation: u64,
    pub expected_applied: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedProjection {
    pub fixture_banner: bool,
    pub live_network_enabled: bool,
    pub peer_count: usize,
    pub conversation_count: usize,
    pub message_count: usize,
    pub accessibility_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypedFailure {
    pub code: String,
    pub retryable: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetClass {
    Ios,
    Android,
}

impl TargetClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ios => "ios",
            Self::Android => "android",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Live,
    Fixture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeBoundary {
    profile: Profile,
}

impl From<Profile> for RuntimeBoundary {
    fn from(profile: Profile) -> Self {
        Self { profile }
    }
}

impl RuntimeBoundary {
    #[must_use]
    pub const fn live_network_allowed(self) -> bool {
        matches!(self.profile, Profile::Live)
    }

    #[must_use]
    pub const fn fixture_marker_visible(self) -> bool {
        matches!(self.profile, Profile::Fixture)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    Connected,
    Reconnecting,
    Failed,
}

impl SessionPhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::Reconnecting => "reconnecting",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BearerKind {
    Tcp,
    BluetoothRnode,
    AndroidUsb,
}

impl BearerKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::BluetoothRnode => "bluetooth-rnode",
            Self::AndroidUsb => "android-usb",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BearerState {
    Connected,
    Disconnected,
    Reconnecting,
    Unavailable,
    Unverified,
}

impl std::fmt::Display for BearerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Connected => "connected",
            Self::Disconnected => "disconnected",
            Self::Reconnecting => "reconnecting",
            Self::Unavailable => "unavailable",
            Self::Unverified => "unverified",
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMethod {
    Direct,
    Propagated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceState {
    Durable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportEvidence {
    Accepted,
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PropagationEvidence {
    Uploaded,
    None,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryEvidence {
    Pending,
    Delivered,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Idle,
    Complete,
    Failed,
}
