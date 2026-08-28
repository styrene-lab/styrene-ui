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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Live,
    Fixture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    Connected,
    Reconnecting,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BearerKind {
    Tcp,
    BluetoothRnode,
    AndroidUsb,
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
