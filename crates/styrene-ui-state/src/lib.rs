//! Renderer-neutral presentation state for Styrene Dioxus applications.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

pub const MAX_IDENTITY_BACKUP_PROTECTION_BYTES: usize = 1024;

#[derive(Clone, Eq, PartialEq)]
pub struct IdentityBackupProtection(Vec<u8>);

impl IdentityBackupProtection {
    /// Create one bounded protection input for an identity recovery operation.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityRecoveryFailure::ProtectionRequired`] for an empty
    /// value and [`IdentityRecoveryFailure::ProtectionTooLarge`] when the UTF-8
    /// bytes exceed [`MAX_IDENTITY_BACKUP_PROTECTION_BYTES`].
    pub fn new(value: String) -> Result<Self, IdentityRecoveryFailure> {
        if value.is_empty() {
            return Err(IdentityRecoveryFailure::ProtectionRequired);
        }
        if value.len() > MAX_IDENTITY_BACKUP_PROTECTION_BYTES {
            return Err(IdentityRecoveryFailure::ProtectionTooLarge);
        }
        Ok(Self(value.into_bytes()))
    }

    #[must_use]
    pub fn into_bytes(mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

impl Drop for IdentityBackupProtection {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

impl fmt::Debug for IdentityBackupProtection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IdentityBackupProtection")
            .field("bytes", &"[REDACTED]")
            .field("len", &self.0.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityRecoveryPhase {
    Idle,
    Creating,
    Exporting,
    Sharing,
    SharePresented,
    Selecting,
    Restoring,
    Restored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityRecoveryFailure {
    ProtectionRequired,
    ProtectionMismatch,
    ProtectionTooLarge,
    ArtifactTooLarge,
    InvalidBackup,
    AuthenticationFailed,
    CustodyUnavailable,
    IdentityConflict,
    UnsupportedBackend,
    PickerCancelled,
    PickerUnavailable,
    PickerReadFailed,
    ShareUnavailable,
    SharePresentationFailed,
    SessionUnavailable,
}

impl IdentityRecoveryFailure {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProtectionRequired => "protection_required",
            Self::ProtectionMismatch => "protection_mismatch",
            Self::ProtectionTooLarge => "protection_too_large",
            Self::ArtifactTooLarge => "artifact_too_large",
            Self::InvalidBackup => "invalid_backup",
            Self::AuthenticationFailed => "authentication_failed",
            Self::CustodyUnavailable => "custody_unavailable",
            Self::IdentityConflict => "identity_conflict",
            Self::UnsupportedBackend => "unsupported_backend",
            Self::PickerCancelled => "picker_cancelled",
            Self::PickerUnavailable => "picker_unavailable",
            Self::PickerReadFailed => "picker_read_failed",
            Self::ShareUnavailable => "share_unavailable",
            Self::SharePresentationFailed => "share_presentation_failed",
            Self::SessionUnavailable => "session_unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityRecoveryState {
    pub phase: IdentityRecoveryPhase,
    pub failure: Option<IdentityRecoveryFailure>,
    pub restore_available: bool,
}

impl Default for IdentityRecoveryState {
    fn default() -> Self {
        Self { phase: IdentityRecoveryPhase::Idle, failure: None, restore_available: false }
    }
}

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
    /// The operator's contact book: favourites, bookmarks, aliases, and
    /// per-contact delivery preferences. Absent in fixtures recorded before
    /// the contact-centric shell; those load with an empty book.
    #[serde(default)]
    pub contact_book: ContactBook,
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
    ToggleFavourite {
        contact_id: String,
    },
    ToggleBookmark {
        contact_id: String,
    },
    SetAlias {
        contact_id: String,
        alias: Option<String>,
    },
    SetDeliveryPreference {
        contact_id: String,
        preference: DeliveryPreference,
    },
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

/// Bound a destination candidate for the New Message field.
///
/// Surrounding whitespace is never part of an LXMF destination, so it is
/// removed before the byte bound is applied. Trimming afterwards would let a
/// pasted or scanned candidate with leading whitespace lose its final
/// characters and be reported as incomplete.
#[must_use]
pub fn bounded_destination_input(value: &str) -> String {
    bounded_input(value.trim(), LXMF_DESTINATION_INPUT_MAX_BYTES)
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
    /// The announcing identity, when known. Empty when the backend has not
    /// resolved an identity for this destination, in which case the
    /// destination hash stands in for identity grouping.
    #[serde(default)]
    pub identity_hash: String,
    /// RNS path length in hops, when a path is known.
    #[serde(default)]
    pub hops: Option<u8>,
    /// The interface kind carrying this peer's freshest announce, when known.
    #[serde(default)]
    pub interface_kind: Option<String>,
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

/// What an announced destination lets the operator do, derived from its
/// aspect. One identity can hold several roles at once; the projection
/// groups announces by identity before roles are derived.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactRole {
    /// `lxmf.delivery`. Message, with receipts. The only role that belongs
    /// in Messages and Contacts by default.
    Person,
    /// `nomadnetwork.node`. Browse pages and files.
    PageHost,
    /// `lxmf.propagation`. Select as propagation node, sync.
    Relay,
    /// Reserved for the roadmap tunnel capability. Nothing produces this yet.
    TunnelPeer,
    /// Any other announced aspect. Shown raw, with no verb offered.
    Unknown,
}

impl ContactRole {
    #[must_use]
    pub fn from_aspect(aspect: &str) -> Self {
        match aspect {
            "lxmf.delivery" => Self::Person,
            "nomadnetwork.node" => Self::PageHost,
            "lxmf.propagation" => Self::Relay,
            _ => Self::Unknown,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::PageHost => "page_host",
            Self::Relay => "relay",
            Self::TunnelPeer => "tunnel_peer",
            Self::Unknown => "unknown",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Person => "Person",
            Self::PageHost => "Page host",
            Self::Relay => "Relay",
            Self::TunnelPeer => "Tunnel peer",
            Self::Unknown => "Unknown",
        }
    }
}

/// How a contact is reachable right now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkMode {
    /// A known RNS path exists, with a hop count.
    RnsDirect,
    /// Reachable only by handing off to a propagation node.
    ViaNode,
    /// Reachable over a direct tunnel.
    Tunnel,
    /// No known path, node, or tunnel right now.
    Unreachable,
}

impl LinkMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RnsDirect => "rns_direct",
            Self::ViaNode => "via_node",
            Self::Tunnel => "tunnel",
            Self::Unreachable => "unreachable",
        }
    }
}

/// The operator's delivery policy for one contact. See `delivery-policy`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryPreference {
    /// Deliver now when reachable; hand to a node otherwise. The default.
    #[default]
    DirectThenNode,
    /// Always hand off to the selected propagation node.
    AlwaysViaNode,
}

impl DeliveryPreference {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DirectThenNode => "direct_then_node",
            Self::AlwaysViaNode => "always_via_node",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "direct_then_node" => Some(Self::DirectThenNode),
            "always_via_node" => Some(Self::AlwaysViaNode),
            _ => None,
        }
    }
}

/// One identity, the destinations it has announced, and what the operator
/// has decided about it. Messages, the contact sheet, and the thread header
/// all render from this projection rather than from raw peers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contact {
    /// The identity hash, or the destination hash when identity is unknown.
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
    pub roles: Vec<ContactRole>,
    pub delivery_destination: Option<String>,
    /// Every announced destination for this identity, as (aspect, hash).
    pub destinations: Vec<(String, String)>,
    pub link: LinkMode,
    pub hops: Option<u8>,
    pub interface_kind: Option<String>,
    pub last_seen: i64,
    pub age_secs: u64,
    pub announce_count: u32,
    pub favourite: bool,
    pub bookmarked: bool,
    pub has_conversation: bool,
    pub unread_count: u32,
    pub delivery_preference: DeliveryPreference,
}

/// The operator-built lists and preferences that `project_contacts` cannot
/// derive from announces alone. Keyed by contact id (see [`Contact::id`]).
/// Every field defaults so a store persisted before a field existed still
/// loads.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContactBook {
    #[serde(default)]
    pub favourites: BTreeSet<String>,
    #[serde(default)]
    pub bookmarks: BTreeSet<String>,
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub delivery_preferences: BTreeMap<String, DeliveryPreference>,
}

const CONTACT_ROLE_ORDER: [ContactRole; 5] = [
    ContactRole::Person,
    ContactRole::PageHost,
    ContactRole::Relay,
    ContactRole::TunnelPeer,
    ContactRole::Unknown,
];

fn peer_group_key(peer: &Peer) -> String {
    if peer.identity_hash.is_empty() {
        format!("destination:{}", peer.destination_hash)
    } else {
        format!("identity:{}", peer.identity_hash)
    }
}

/// Group announced peers by identity and project them into contacts.
///
/// Peers group by [`Peer::identity_hash`] when it is non-empty, and by
/// [`Peer::destination_hash`] otherwise, so an identity that has not been
/// resolved yet still becomes its own contact rather than being dropped.
///
/// `link` reflects only what this projection can see today: [`LinkMode::Tunnel`]
/// is never produced (the roadmap capability that would produce it does not
/// exist yet), [`LinkMode::RnsDirect`] is used whenever a hop count is known,
/// and everything else falls back to [`LinkMode::Unreachable`].
/// [`LinkMode::ViaNode`] is intentionally left unset here; it depends on
/// whether a propagation node is selected and ready, which is session state
/// this projection does not see, so the session sets it after calling this
/// function.
#[must_use]
pub fn project_contacts(
    peers: &[Peer],
    conversations: &[Conversation],
    book: &ContactBook,
) -> Vec<Contact> {
    let mut groups: BTreeMap<String, Vec<&Peer>> = BTreeMap::new();
    for peer in peers {
        groups.entry(peer_group_key(peer)).or_default().push(peer);
    }

    let mut contacts: Vec<Contact> = groups
        .into_values()
        .map(|group| project_contact_group(&group, conversations, book))
        .collect();

    // A peer the operator has messaged but never heard announce still belongs
    // in Contacts; the daemon keys the conversation by its delivery destination.
    let announced: BTreeSet<&str> =
        peers.iter().map(|peer| peer.destination_hash.as_str()).collect();
    for conversation in conversations {
        if announced.contains(conversation.peer_hash.as_str()) {
            continue;
        }
        if contacts.iter().any(|contact| contact.id == conversation.peer_hash) {
            continue;
        }
        contacts.push(conversation_only_contact(conversation, book));
    }

    contacts.sort_by_key(|contact| {
        (!(contact.has_conversation || contact.favourite), std::cmp::Reverse(contact.last_seen))
    });
    contacts
}

fn conversation_only_contact(conversation: &Conversation, book: &ContactBook) -> Contact {
    let id = conversation.peer_hash.clone();
    let alias = book.aliases.get(&id).cloned();
    let name = alias.clone().unwrap_or_else(|| format!("Peer {}", &id[..id.len().min(8)]));
    Contact {
        favourite: book.favourites.contains(&id),
        bookmarked: book.bookmarks.contains(&id),
        delivery_preference: book.delivery_preferences.get(&id).copied().unwrap_or_default(),
        name,
        alias,
        roles: vec![ContactRole::Person],
        delivery_destination: Some(id.clone()),
        destinations: vec![("lxmf.delivery".to_owned(), id.clone())],
        id,
        link: LinkMode::Unreachable,
        hops: None,
        interface_kind: None,
        last_seen: 0,
        age_secs: 0,
        announce_count: 0,
        has_conversation: true,
        unread_count: conversation.unread_count,
    }
}

fn project_contact_group(
    group: &[&Peer],
    conversations: &[Conversation],
    book: &ContactBook,
) -> Contact {
    let roles: Vec<ContactRole> = CONTACT_ROLE_ORDER
        .into_iter()
        .filter(|role| group.iter().any(|peer| ContactRole::from_aspect(&peer.aspect) == *role))
        .collect();

    let identity_hash =
        group.iter().find(|peer| !peer.identity_hash.is_empty()).map(|peer| &peer.identity_hash);
    let id = identity_hash.cloned().unwrap_or_else(|| {
        group.first().map_or_else(String::new, |peer| peer.destination_hash.clone())
    });

    let delivery_destination = group
        .iter()
        .find(|peer| ContactRole::from_aspect(&peer.aspect) == ContactRole::Person)
        .map(|peer| peer.destination_hash.clone());

    let alias = book.aliases.get(&id).cloned();
    let name = alias.clone().unwrap_or_else(|| {
        group
            .iter()
            .filter(|peer| ContactRole::from_aspect(&peer.aspect) == ContactRole::Person)
            .find_map(|peer| peer.display_name.clone().filter(|name| !name.is_empty()))
            .or_else(|| {
                group
                    .iter()
                    .find_map(|peer| peer.display_name.clone().filter(|name| !name.is_empty()))
            })
            .unwrap_or_else(|| {
                let basis = delivery_destination.as_deref().unwrap_or(id.as_str());
                format!("Peer {}", &basis[..basis.len().min(8)])
            })
    });

    let destinations: Vec<(String, String)> =
        group.iter().map(|peer| (peer.aspect.clone(), peer.destination_hash.clone())).collect();

    let last_seen = group.iter().map(|peer| peer.observed_at).max().unwrap_or_default();
    let age_secs = group.iter().map(|peer| peer.age_secs).min().unwrap_or_default();
    let announce_count = group.iter().map(|peer| peer.announce_count).sum();

    let freshest = group.iter().max_by_key(|peer| peer.observed_at).copied();
    let hops = freshest.and_then(|peer| peer.hops);
    let interface_kind = freshest.and_then(|peer| peer.interface_kind.clone());

    let link = if hops.is_some() { LinkMode::RnsDirect } else { LinkMode::Unreachable };

    let has_conversation = delivery_destination
        .as_deref()
        .is_some_and(|destination| conversations.iter().any(|c| c.peer_hash == destination));
    let unread_count = delivery_destination
        .as_deref()
        .and_then(|destination| conversations.iter().find(|c| c.peer_hash == destination))
        .map_or(0, |conversation| conversation.unread_count);

    Contact {
        favourite: book.favourites.contains(&id),
        bookmarked: book.bookmarks.contains(&id),
        delivery_preference: book.delivery_preferences.get(&id).copied().unwrap_or_default(),
        id,
        name,
        alias,
        roles,
        delivery_destination,
        destinations,
        link,
        hops,
        interface_kind,
        last_seen,
        age_secs,
        announce_count,
        has_conversation,
        unread_count,
    }
}

/// The operator-built lists a contact-centric shell renders. See
/// `openspec/changes/contact-centric-shell/proposal.md`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactLists {
    /// People the operator has messaged or favourited.
    pub contacts: Vec<Contact>,
    /// Page hosts the operator has bookmarked.
    pub pages: Vec<Contact>,
    /// Everything announced, for the Network directory.
    pub directory: Vec<Contact>,
}

#[must_use]
pub fn contact_lists(contacts: &[Contact]) -> ContactLists {
    let list_contacts = contacts
        .iter()
        .filter(|contact| {
            contact.roles.contains(&ContactRole::Person)
                && (contact.has_conversation || contact.favourite)
        })
        .cloned()
        .collect();
    let pages = contacts
        .iter()
        .filter(|contact| contact.roles.contains(&ContactRole::PageHost) && contact.bookmarked)
        .cloned()
        .collect();
    ContactLists { contacts: list_contacts, pages, directory: contacts.to_vec() }
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
    pub retry_ineligibility_reason: Option<MessageRetryIneligibilityReason>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRetryIneligibilityReason {
    Inbound,
    MissingOutboundRoute,
    LifecycleState,
    CanonicalWireUnavailable,
    AttemptLimitReached,
    #[default]
    Unknown,
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
    pub readiness: PropagationReadiness,
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
    pub trigger_capabilities: Vec<PropagationTriggerSource>,
    pub active_trigger: Option<PropagationTriggerSource>,
    pub active_sync_started_at: Option<i64>,
    pub last_synchronization: Option<PropagationSynchronization>,
    pub cooldown_remaining_secs: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PropagationReadiness {
    Unselected,
    Ready,
    Unavailable,
    Inactive,
    InvalidMetadata,
}

impl PropagationReadiness {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unselected => "unselected",
            Self::Ready => "ready",
            Self::Unavailable => "unavailable",
            Self::Inactive => "inactive",
            Self::InvalidMetadata => "invalid_metadata",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PropagationTriggerSource {
    InitialConnection,
    Reconnect,
    ForegroundOpportunity,
    GrantedBackgroundOpportunity,
    Manual,
}

impl PropagationTriggerSource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InitialConnection => "initial_connection",
            Self::Reconnect => "reconnect",
            Self::ForegroundOpportunity => "foreground_opportunity",
            Self::GrantedBackgroundOpportunity => "granted_background_opportunity",
            Self::Manual => "manual",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PropagationTerminalOutcome {
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
}

impl PropagationTerminalOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PropagationSynchronization {
    pub trigger: PropagationTriggerSource,
    pub started_at: i64,
    pub finished_at: i64,
    pub outcome: PropagationTerminalOutcome,
    pub new_messages: u32,
}

impl PropagationUpdate {
    #[must_use]
    pub fn from_fixture(fixture: &MobileFixture) -> Self {
        Self {
            generation: fixture.generation,
            selected_destination: fixture.propagation.selected_destination.clone(),
            readiness: if fixture.propagation.selected_destination.is_none() {
                PropagationReadiness::Unselected
            } else if fixture.propagation.ready {
                PropagationReadiness::Ready
            } else {
                PropagationReadiness::Unavailable
            },
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
            trigger_capabilities: Vec::new(),
            active_trigger: None,
            active_sync_started_at: None,
            last_synchronization: None,
            cooldown_remaining_secs: 0,
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
    Connecting,
    Disconnected,
    Reconnecting,
    Unavailable,
    Unverified,
}

impl std::fmt::Display for BearerState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Connected => "connected",
            Self::Connecting => "connecting",
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
