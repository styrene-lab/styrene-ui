//! Renderer-neutral presentation state for Styrene Dioxus applications.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobileMinimumCorpus {
    pub schema_version: u32,
    pub corpus: String,
    pub target_classes: Vec<TargetClass>,
    pub required_accessibility_ids: Vec<String>,
    pub fixtures: Vec<MobileFixture>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
pub struct MobileAction {
    pub generation: u64,
    pub kind: MobileActionKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MobileActionKind {
    ApplyEndpoint {
        endpoint: String,
    },
    SetActiveConversation {
        peer_hash: Option<String>,
    },
    StartConversation {
        peer_hash: String,
    },
    SetIdentityDisplayName {
        display_name: String,
    },
    SaveDraft {
        peer_hash: String,
        content: String,
        base_revision: u64,
    },
    SendMessage {
        peer_hash: String,
        content: String,
        requested_method: DeliveryMethod,
        draft_revision: u64,
    },
    RetryMessage {
        message_id: String,
    },
    SelectPropagationNode {
        destination_hash: Option<String>,
    },
    SyncPropagation,
}

pub const LXMF_DESTINATION_INPUT_MAX_BYTES: usize = 32;
pub const PEER_SEARCH_INPUT_MAX_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestinationEntryConstraint {
    Empty,
    Incomplete,
    Ready,
    Oversized,
}

impl DestinationEntryConstraint {
    #[must_use]
    pub const fn permits_submission(self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[must_use]
pub fn destination_entry_constraint(value: &str) -> DestinationEntryConstraint {
    match value.trim().len() {
        0 => DestinationEntryConstraint::Empty,
        LXMF_DESTINATION_INPUT_MAX_BYTES => DestinationEntryConstraint::Ready,
        length if length > LXMF_DESTINATION_INPUT_MAX_BYTES => {
            DestinationEntryConstraint::Oversized
        }
        _ => DestinationEntryConstraint::Incomplete,
    }
}

#[must_use]
pub fn bounded_destination_input(value: &str) -> String {
    bounded_input(value, LXMF_DESTINATION_INPUT_MAX_BYTES)
}

#[must_use]
pub fn bounded_peer_search_input(value: &str) -> String {
    bounded_input(value, PEER_SEARCH_INPUT_MAX_BYTES)
}

fn bounded_input(value: &str, accepted_bytes: usize) -> String {
    let retained_bytes = accepted_bytes.saturating_add(1);
    if value.len() <= retained_bytes {
        return value.to_owned();
    }

    let mut end = 0;
    for (index, character) in value.char_indices() {
        let character_end = index + character.len_utf8();
        if character_end > retained_bytes {
            break;
        }
        end = character_end;
    }
    value[..end].to_owned()
}

#[must_use]
pub fn start_conversation_action(generation: u64, value: &str) -> Option<MobileAction> {
    destination_entry_constraint(value).permits_submission().then(|| {
        MobileAction::new(
            generation,
            MobileActionKind::StartConversation { peer_hash: value.trim().to_owned() },
        )
    })
}

#[must_use]
pub fn peer_matches_search(peer: &Peer, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty()
        || peer.destination_hash.to_ascii_lowercase().contains(&query)
        || peer.aspect.to_ascii_lowercase().contains(&query)
        || peer
            .display_name
            .as_deref()
            .is_some_and(|name| name.to_ascii_lowercase().contains(&query))
}

impl MobileAction {
    #[must_use]
    pub const fn new(generation: u64, kind: MobileActionKind) -> Self {
        Self { generation, kind }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MobileStore {
    snapshot: MobileFixture,
    local_announce_outcome: Option<LocalAnnounceOutcome>,
    propagation: PropagationUpdate,
}

impl MobileStore {
    #[must_use]
    pub fn new(snapshot: MobileFixture) -> Self {
        let propagation = PropagationUpdate::from_fixture(&snapshot);
        Self { snapshot, local_announce_outcome: None, propagation }
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
        self.snapshot.propagation.ready = false;
        self.propagation.generation = generation;
        self.propagation.ready = false;
        self.propagation.sync_state = SyncState::Idle;
        self.propagation.progress = None;
        self.propagation.failure = None;
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
        self.propagation = PropagationUpdate::from_fixture(&self.snapshot);
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
                && current.observed_at > peer.observed_at
            {
                *peer = current.clone();
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

    pub fn apply_draft(&mut self, draft: Conversation) -> ApplyResult {
        if let Some(current) = self
            .snapshot
            .conversations
            .iter_mut()
            .find(|conversation| conversation.peer_hash == draft.peer_hash)
        {
            if draft.draft_revision < current.draft_revision {
                return ApplyResult::IgnoredStale;
            }
            current.draft = draft.draft;
            current.draft_revision = draft.draft_revision;
        } else {
            self.snapshot.conversations.push(draft);
        }
        ApplyResult::Applied
    }

    pub fn apply_message_snapshot(&mut self, snapshot: MessageSnapshot) -> ApplyResult {
        if snapshot.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }
        self.snapshot.messages = canonical_messages(snapshot.messages);
        self.snapshot.conversations = snapshot.conversations;
        ApplyResult::Applied
    }

    pub fn apply_message_event(&mut self, event: MessageEvent) -> ApplyResult {
        if event.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }
        if let Some(current) =
            self.snapshot.messages.iter_mut().find(|message| message.id == event.message.id)
        {
            *current = event.message;
        } else {
            self.snapshot.messages.push(event.message);
        }
        ApplyResult::Applied
    }

    pub fn apply_send_outcome(&mut self, outcome: SendOutcome) -> ApplyResult {
        if outcome.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }
        let peer_hash = outcome.message.peer_hash.clone();
        self.apply_message_event(MessageEvent {
            generation: outcome.generation,
            message: outcome.message,
        });
        if outcome.draft_clear == DraftClearDisposition::Cleared
            && let Some(submitted_revision) = outcome.submitted_draft_revision
            && let Some(conversation) = self
                .snapshot
                .conversations
                .iter_mut()
                .find(|conversation| conversation.peer_hash == peer_hash)
            && conversation.draft_revision == submitted_revision
        {
            conversation.draft.clear();
        }
        ApplyResult::Applied
    }

    pub fn apply_propagation_update(&mut self, update: PropagationUpdate) -> ApplyResult {
        if update.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }
        self.snapshot.propagation = Propagation {
            selected_destination: update.selected_destination.clone(),
            ready: update.ready,
            sync_state: update.sync_state,
            new_messages: update.new_messages,
            failure: update.failure.clone(),
        };
        self.propagation = update;
        ApplyResult::Applied
    }

    pub fn apply_bearer_event(&mut self, event: BearerEvent) -> ApplyResult {
        if event.generation != self.snapshot.generation {
            return ApplyResult::IgnoredStale;
        }
        if let Some(bearer) =
            self.snapshot.bearers.iter_mut().find(|bearer| bearer.kind == event.bearer.kind)
        {
            *bearer = event.bearer;
        } else {
            self.snapshot.bearers.push(event.bearer);
        }
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
    pub const fn propagation(&self) -> &PropagationUpdate {
        &self.propagation
    }

    #[must_use]
    pub fn messaging_available(&self) -> bool {
        self.snapshot.bearers.iter().any(|bearer| bearer.state == BearerState::Connected)
    }

    #[must_use]
    pub fn operational_summary(&self) -> OperationalSummary {
        let mut route_observed = 0usize;
        let mut route_unknown = 0usize;
        for attempt in self.snapshot.messages.iter().flat_map(|message| &message.details.attempts) {
            match attempt.route.outcome {
                MessageRouteOutcome::Observed => route_observed = route_observed.saturating_add(1),
                MessageRouteOutcome::Unknown => route_unknown = route_unknown.saturating_add(1),
            }
        }

        OperationalSummary {
            runtime: self.snapshot.session.runtime,
            phase: self.snapshot.session.phase,
            connected_bearers: self
                .snapshot
                .bearers
                .iter()
                .filter(|bearer| bearer.state == BearerState::Connected)
                .count(),
            bearer_count: self.snapshot.bearers.len(),
            peer_count: self.snapshot.peers.len(),
            unread_count: self
                .snapshot
                .conversations
                .iter()
                .fold(0u32, |total, conversation| total.saturating_add(conversation.unread_count)),
            loaded_route_observed: route_observed,
            loaded_route_unknown: route_unknown,
            propagation_selected: self.propagation.selected_destination.is_some(),
            propagation_ready: self.propagation.ready,
            propagation_sync_state: self.propagation.sync_state,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OperationalSummary {
    pub runtime: SessionRuntime,
    pub phase: SessionPhase,
    pub connected_bearers: usize,
    pub bearer_count: usize,
    pub peer_count: usize,
    pub unread_count: u32,
    /// Counts only route observations in the currently loaded message projection.
    pub loaded_route_observed: usize,
    /// Counts only explicitly unknown routes in the currently loaded message projection.
    pub loaded_route_unknown: usize,
    pub propagation_selected: bool,
    pub propagation_ready: bool,
    pub propagation_sync_state: SyncState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    #[serde(default)]
    pub runtime: SessionRuntime,
    pub phase: SessionPhase,
    pub identity_hash: String,
    #[serde(default)]
    pub display_name: String,
    pub endpoint: Option<String>,
    pub failure: Option<TypedFailure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custody: Option<IdentityCustody>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRuntime {
    #[default]
    Ready,
    Failed,
    Stopped,
}

impl SessionRuntime {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityCustody {
    pub requested_backend: IdentityCustodyBackend,
    pub active_backend: Option<IdentityCustodyBackend>,
    pub protection: Option<IdentityCustodyProtection>,
    pub authentication: IdentityCustodyAuthentication,
    pub availability: IdentityCustodyAvailability,
    pub downgrade: IdentityCustodyDowngrade,
    pub failure: Option<TypedFailure>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityCustodyBackend {
    Keychain,
    AndroidKeystore,
    EncryptedFile,
    PlaintextFile,
}

impl IdentityCustodyBackend {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keychain => "keychain",
            Self::AndroidKeystore => "android_keystore",
            Self::EncryptedFile => "encrypted_file",
            Self::PlaintextFile => "plaintext_file",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityCustodyProtection {
    PlatformProtected,
    EncryptedAtRest,
    DevelopmentPlaintext,
}

impl IdentityCustodyProtection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlatformProtected => "platform_protected",
            Self::EncryptedAtRest => "encrypted_at_rest",
            Self::DevelopmentPlaintext => "development_plaintext",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityCustodyAuthentication {
    DeviceAuthentication,
    HostKeyMaterial,
    None,
}

impl IdentityCustodyAuthentication {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeviceAuthentication => "device_authentication",
            Self::HostKeyMaterial => "host_key_material",
            Self::None => "none",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityCustodyAvailability {
    Available,
    Unavailable,
}

impl IdentityCustodyAvailability {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityCustodyDowngrade {
    None,
    ActiveBackendMismatch,
}

impl IdentityCustodyDowngrade {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ActiveBackendMismatch => "active_backend_mismatch",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bearer {
    pub kind: BearerKind,
    pub state: BearerState,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BearerEvent {
    pub generation: u64,
    pub bearer: Bearer,
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

#[derive(Clone, Debug, PartialEq)]
pub struct MessageSnapshot {
    pub generation: u64,
    pub conversations: Vec<Conversation>,
    pub messages: Vec<Message>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessageEvent {
    pub generation: u64,
    pub message: Message,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftClearDisposition {
    NotRequested,
    Cleared,
    Superseded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SendOutcome {
    pub generation: u64,
    pub message: Message,
    pub submitted_draft_revision: Option<u64>,
    pub draft_clear: DraftClearDisposition,
}

fn canonical_messages(messages: Vec<Message>) -> Vec<Message> {
    let mut canonical = Vec::with_capacity(messages.len());
    for message in messages {
        if let Some(current) =
            canonical.iter_mut().find(|current: &&mut Message| current.id == message.id)
        {
            *current = message;
        } else {
            canonical.push(message);
        }
    }
    canonical
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
    pub draft_revision: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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
    #[serde(default)]
    pub lifecycle: Option<MessageLifecycle>,
    #[serde(default)]
    pub details: MessageDetails,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessageDetails {
    pub projection_complete: bool,
    pub source_hash: String,
    pub destination_hash: String,
    pub timestamp: i64,
    pub lxmf_timestamp: Option<f64>,
    pub title: Option<String>,
    pub status: String,
    pub terminal_detail: Option<String>,
    pub is_outgoing: bool,
    pub delivery_method: Option<String>,
    pub requested_delivery_method: Option<String>,
    pub actual_delivery_method: Option<String>,
    pub fallback_reason: Option<String>,
    pub correlation_id: Option<String>,
    pub attempts: Vec<MessageAttempt>,
    pub propagation_correlations: Vec<MessagePropagationCorrelation>,
    pub read: bool,
    pub attachment_info: Option<MessageAttachment>,
    pub attachments: Vec<MessageAttachment>,
    pub authentication: MessageAuthentication,
    pub stamp_state: MessageStampState,
    pub stamp_value: Option<u32>,
    pub stamp_cost: Option<u32>,
    pub delivery_evidence: Vec<MessageDeliveryObservation>,
    pub retry_eligible: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessageAttempt {
    pub message_id: String,
    pub number: u32,
    pub started_unix_ms: i64,
    pub deadline_unix_ms: i64,
    pub state: String,
    pub bearer: Option<String>,
    pub route: MessageRouteObservation,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessageRouteObservation {
    pub outcome: MessageRouteOutcome,
    pub connection_generation: Option<u64>,
    pub observed_at: Option<i64>,
    pub next_hop: Option<String>,
    pub hops: Option<u32>,
    pub stale: bool,
    pub interface: Option<MessageInterfaceObservation>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessageInterfaceObservation {
    pub id: String,
    pub kind: String,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRouteOutcome {
    Observed,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessagePropagationCorrelation {
    pub relation: String,
    pub transient_id: String,
    pub attempt_id: Option<String>,
    pub peer_hash: Option<String>,
    pub state: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessageDeliveryObservation {
    pub kind: MessageDeliveryKind,
    pub hash: String,
    pub representation: String,
    pub state: MessageDeliveryState,
    pub outcome: Option<String>,
    pub attempt: Option<u32>,
    pub correlation_id: Option<String>,
    pub observed_at: i64,
    pub terminal_at: Option<i64>,
    pub transferred_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub progress: Option<u8>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDeliveryKind {
    PacketReceipt,
    ResourceCompletion,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDeliveryState {
    Tracked,
    Completed,
    Failed,
    Cancelled,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageAuthentication {
    Verified,
    Invalid,
    UnknownIdentity,
    NotApplicable,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStampState {
    Verified,
    Invalid,
    NotApplicable,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessageAttachment {
    pub ordinal: u8,
    pub id: String,
    pub name: String,
    pub content_type: String,
    pub size: u64,
    pub checksum: String,
    pub availability: String,
    pub integrity: String,
    pub transfer: Option<MessageAttachmentTransfer>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MessageAttachmentTransfer {
    pub message_id: String,
    pub transfer_id: String,
    pub resource_hash: Option<String>,
    pub representation: String,
    pub direction: String,
    pub state: String,
    pub transferred: u64,
    pub total: u64,
    pub checksum_verified: bool,
    pub cancellable: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageLifecycle {
    Queued,
    Sending,
    Sent,
    Delivered,
    Failed,
    Cancelled,
    Expired,
    Rejected,
    Unknown,
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
pub struct PropagationProgress {
    pub attempt_id: String,
    pub received_count: u64,
    pub received_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PropagationPolicy {
    pub transfer_limit_kb: u64,
    pub sync_limit_kb: u64,
    pub stamp_cost: u32,
    pub stamp_flexibility: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PropagationCandidate {
    pub destination_hash: String,
    pub active: bool,
    pub observed_at: i64,
    pub age_secs: u64,
    pub policy: Option<PropagationPolicy>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PropagationUpdate {
    pub generation: u64,
    pub selected_destination: Option<String>,
    pub ready: bool,
    pub sync_state: SyncState,
    pub new_messages: u32,
    pub failure: Option<TypedFailure>,
    pub automatic_sync_enabled: bool,
    pub automatic_sync_cooldown_secs: u64,
    pub sync_deadline_secs: u64,
    pub progress: Option<PropagationProgress>,
    pub candidates: Vec<PropagationCandidate>,
    pub selected_policy: Option<PropagationPolicy>,
}

impl PropagationUpdate {
    #[must_use]
    pub fn from_fixture(fixture: &MobileFixture) -> Self {
        Self {
            generation: fixture.generation,
            selected_destination: fixture.propagation.selected_destination.clone(),
            ready: fixture.propagation.ready,
            sync_state: fixture.propagation.sync_state,
            new_messages: fixture.propagation.new_messages,
            failure: fixture.propagation.failure.clone(),
            automatic_sync_enabled: false,
            automatic_sync_cooldown_secs: 0,
            sync_deadline_secs: 0,
            progress: None,
            candidates: Vec::new(),
            selected_policy: None,
        }
    }
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
    Stopped,
    Starting,
    Offline,
    Connecting,
    Connected,
    Reconnecting,
    Degraded,
    Failed,
}

impl SessionPhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Starting => "starting",
            Self::Offline => "offline",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Reconnecting => "reconnecting",
            Self::Degraded => "degraded",
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
    Opportunistic,
    Propagated,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceState {
    Durable,
    Unknown,
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
    InProgress,
    Complete,
    Failed,
}

impl SyncState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::InProgress => "in_progress",
            Self::Complete => "complete",
            Self::Failed => "failed",
        }
    }
}
