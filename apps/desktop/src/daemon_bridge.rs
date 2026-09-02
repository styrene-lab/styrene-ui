//! Daemon session adapter over the shared IPC client and session profiles.
//!
//! Live sessions negotiate against an existing daemon endpoint. Embedded
//! sessions start a `styrened` runtime in-process and reach it over the
//! session's private socket. Both take the same typed client path; this
//! module only maps canonical records onto desktop events and commands.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, broadcast, mpsc};
use tokio::time::Duration;
use tracing::{Instrument, debug, info, info_span, warn};

use rmpv::Value as MpValue;
use styrene_ipc::types::ProfileInfo;
use styrene_ipc::types::{
    ConversationInfo, DaemonStatusInfo, DeviceInfo, ExecResult, IdentityInfo, LinkEvent,
    LinkSnapshot, MessageInfo, NetworkOperationInfo, NetworkOperationKind, ObservationMetadata,
    PathInfo, PropagationQuery, PropagationSnapshot, RebootResult, RemoteStatusInfo,
    RequestObservationInfo, ResourceTransferInfo, RouteEventInfo, StandardPropagationSnapshot,
    StartNetworkOperationInfo, StartRequestInfo,
};
use styrene_ipc_client::{Client, ClientError, EventFrame, EventTopic};
use styrene_ipc_wire::MessageType;
use styrene_session::{EmbeddedConfig, ManagedTarget, Session, SessionProfile};

/// A single entry from the path table — routing info for one destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathTableEntry {
    pub destination_hash: String,
    pub hops: u8,
    pub next_hop: String,
    pub interface: String,
    pub expires: Option<i64>,
    pub observation: ObservationMetadata,
}

/// Exact typed interface projection from the Unix IPC contract.
pub type InterfaceStats = styrene_ipc::types::InterfaceDetail;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageResponse {
    pub page: styrene_ipc::types::PageContent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BrokerDiagnostics {
    pub queue_depth: usize,
    pub in_flight: usize,
    pub completed: u64,
    pub timed_out: u64,
    pub cancelled: u64,
    pub overloaded: u64,
    pub disconnected: u64,
    pub reconnects: u64,
    pub stale_responses: u64,
    pub dropped_responses: u64,
    pub dropped_updates: u64,
    pub last_latency_ms: u64,
}

/// Events pushed from the daemon to the UI.
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    Connected,
    /// The backend's description of the profile its daemon runs from.
    Profile(Box<ProfileInfo>),
    Identity(IdentityInfo),
    Status(DaemonStatusInfo),
    EventGeneration(u64),
    PeerDiscovered(DeviceInfo),
    LocalPageInventory(Vec<styrene_ipc::types::PageInfo>),
    MessageReceived(Box<MessageInfo>),
    MessagingOperation(Box<styrene_ipc::types::MessagingOperationOutcome>),
    LinkObservation(LinkEvent),
    PathTable(Vec<PathTableEntry>),
    RouteLifecycle(RouteEventInfo),
    NetworkOperation(NetworkOperationInfo),
    Request(RequestObservationInfo),
    Resource(ResourceTransferInfo),
    ReconcileRequests {
        dropped: u64,
        connection_generation: u64,
    },
    ReconcileRequired {
        dropped: u64,
        connection_generation: u64,
    },
    StandardPropagationChanged {
        connection_generation: u64,
    },
    Disconnected(String),
}

/// Commands sent from the UI to the daemon.
#[derive(Clone)]
pub enum DaemonCommand {
    SendChat {
        peer_hash: String,
        content: String,
        title: Option<String>,
        delivery_method: String,
        attachments: Vec<styrene_ipc::types::AttachmentInput>,
    },
    LoadDraft {
        peer_hash: String,
    },
    SaveDraft {
        peer_hash: String,
        content: String,
    },
    DiscardDraft {
        peer_hash: String,
    },
    RetryMessage {
        message_id: String,
    },
    CancelMessage {
        message_id: String,
    },
    BrowsePage {
        address: crate::state::PageAddress,
    },
    NavigatePage(styrene_ipc::types::PageNavigationRequest),
    ClosePage {
        session_id: String,
    },
    StartFileDownload(styrene_ipc::types::FileDownloadRequest),
    QueryFileDownload {
        download_id: String,
    },
    CancelFileDownload {
        download_id: String,
    },
    SaveFileDownload {
        download_id: String,
        destination: String,
    },
    RefreshPathTable,
    RefreshInterfaces,
    RefreshLinks,
    RefreshOperations,
    RefreshRequests,
    RefreshResources,
    StartNetworkOperation(StartNetworkOperationInfo),
    CancelNetworkOperation {
        operation_id: String,
    },
    StartRequest(StartRequestInfo),
    CancelRequest {
        request_id: String,
    },
    CancelResource {
        resource_hash: String,
    },
    RefreshPropagation {
        cursor: Option<String>,
    },
    RefreshStandardPropagation,
    LoadConversations {
        cursor: Option<String>,
    },
    LoadMessages {
        peer_hash: String,
        cursor: Option<String>,
    },
    QueryMessage {
        message_id: String,
    },
    FleetStatus {
        destination: String,
    },
    FleetExec {
        destination: String,
        command: String,
        args: Vec<String>,
    },
    FleetReboot {
        destination: String,
        delay: Option<u64>,
    },
    FleetApply {
        destination: String,
        profile_base64: String,
    },
    BlockPeer {
        identity_hash: String,
    },
}

impl DaemonCommand {
    pub fn required_capability(&self) -> Option<&'static str> {
        match self {
            Self::SendChat { .. } => Some("chat.send"),
            Self::LoadDraft { .. } | Self::SaveDraft { .. } | Self::DiscardDraft { .. } => {
                Some("messaging.manage")
            }
            Self::RetryMessage { .. } | Self::CancelMessage { .. } => Some("messaging.lifecycle"),
            Self::BrowsePage { .. }
            | Self::NavigatePage(_)
            | Self::ClosePage { .. }
            | Self::StartFileDownload(_)
            | Self::QueryFileDownload { .. }
            | Self::CancelFileDownload { .. }
            | Self::SaveFileDownload { .. } => Some("page.browse"),
            Self::RefreshPathTable
            | Self::RefreshInterfaces
            | Self::RefreshLinks
            | Self::RefreshOperations
            | Self::RefreshRequests
            | Self::RefreshResources
            | Self::RefreshPropagation { .. }
            | Self::RefreshStandardPropagation
            | Self::LoadConversations { .. }
            | Self::LoadMessages { .. }
            | Self::FleetStatus { .. } => Some("rpc.status"),
            Self::QueryMessage { .. } => Some("messaging.history.read"),
            Self::StartNetworkOperation(request) => match request.kind {
                NetworkOperationKind::Announce => Some("network.announce"),
                NetworkOperationKind::PathRequest => Some("network.path_request"),
                NetworkOperationKind::Probe => Some("network.probe"),
                NetworkOperationKind::LinkOpen => Some("network.link_open"),
                NetworkOperationKind::LinkClose => Some("network.link_close"),
                _ => None,
            },
            Self::CancelNetworkOperation { .. } => None,
            Self::StartRequest(_) => Some("network.request"),
            Self::CancelRequest { .. } => Some("network.request_cancel"),
            Self::CancelResource { .. } => Some("network.resource_cancel"),
            Self::FleetExec { .. } => Some("rpc.exec"),
            Self::FleetReboot { .. } => Some("rpc.reboot"),
            Self::FleetApply { .. } => Some("rpc.fleet_apply"),
            Self::BlockPeer { .. } => Some("policy.update"),
        }
    }
}

/// The Embedded runtime a session started. Shutting it down closes the
/// session, which stops the daemon and removes its private state directory.
pub struct EmbeddedDaemon {
    session: Arc<Mutex<Option<Session>>>,
}

impl EmbeddedDaemon {
    pub(crate) async fn shutdown(self) {
        close_session(&self.session).await;
    }

    #[cfg(test)]
    async fn endpoint(&self) -> Option<std::path::PathBuf> {
        self.session.lock().await.as_ref().map(|session| session.metadata().endpoint.clone())
    }
}

async fn close_session(slot: &Mutex<Option<Session>>) {
    if let Some(mut session) = slot.lock().await.take() {
        session.close().await;
    }
}

/// Typed daemon operations over the shared client, plus the sessions that
/// keep the command and event connections alive.
#[derive(Clone)]
pub struct RequestBroker {
    client: Client,
    command_session: Arc<Mutex<Option<Session>>>,
    event_session: Arc<Mutex<Option<Session>>>,
    reconnects: u64,
    dropped_updates: Arc<AtomicU64>,
}

fn describe(error: ClientError) -> String {
    error.to_string()
}

impl RequestBroker {
    fn new(
        client: Client,
        command_session: Arc<Mutex<Option<Session>>>,
        event_session: Arc<Mutex<Option<Session>>>,
        generation: crate::backend::ConnectionGeneration,
    ) -> Self {
        Self {
            client,
            command_session,
            event_session,
            reconnects: generation.0.saturating_sub(1),
            dropped_updates: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) fn diagnostics(&self) -> BrokerDiagnostics {
        let client = self.client.diagnostics();
        BrokerDiagnostics {
            queue_depth: client.queue_depth,
            in_flight: client.in_flight,
            completed: client.completed,
            timed_out: client.timed_out,
            cancelled: client.cancelled,
            overloaded: client.overloaded,
            disconnected: client.disconnected,
            reconnects: self.reconnects,
            stale_responses: client.stale_responses,
            dropped_responses: client.dropped_responses,
            dropped_updates: self.dropped_updates.load(Ordering::Relaxed),
            last_latency_ms: client.last_latency_ms,
        }
    }

    /// Close both sessions. Idempotent.
    pub(crate) async fn shutdown(&self) {
        close_session(&self.event_session).await;
        close_session(&self.command_session).await;
    }

    pub(crate) async fn send_chat_outcome(
        &self,
        request: &styrene_ipc::types::SendChatRequest,
    ) -> Result<styrene_ipc::types::SendChatOutcome, String> {
        self.client.send_chat_outcome(request).await.map_err(describe)
    }

    pub(crate) async fn draft(
        &self,
        peer_hash: &str,
    ) -> Result<Option<styrene_ipc::types::ConversationDraft>, String> {
        self.client.draft(peer_hash).await.map_err(describe)
    }

    pub(crate) async fn save_draft(
        &self,
        peer_hash: &str,
        content: &str,
    ) -> Result<styrene_ipc::types::ConversationDraft, String> {
        self.client.set_draft(peer_hash, content).await.map_err(describe)
    }

    pub(crate) async fn discard_draft(&self, peer_hash: &str) -> Result<(), String> {
        self.client.clear_draft(peer_hash).await.map_err(describe)
    }

    pub(crate) async fn message_lifecycle(
        &self,
        message_id: &str,
        retry: bool,
    ) -> Result<styrene_ipc::types::MessagingOperationOutcome, String> {
        if retry {
            self.client.retry_message(message_id).await.map_err(describe)
        } else {
            self.client.cancel_message(message_id).await.map_err(describe)
        }
    }

    pub(crate) async fn browse_page(&self, host: &str, path: &str) -> Result<PageResponse, String> {
        let timeout = if host.is_empty() || host == "local" { None } else { Some(25) };
        let page = self.client.browse_page(host, path, timeout).await.map_err(describe)?;
        Ok(PageResponse { page })
    }

    pub(crate) async fn local_page_inventory(
        &self,
    ) -> Result<Vec<styrene_ipc::types::PageInfo>, String> {
        self.client.page_inventory("local", None).await.map_err(describe)
    }

    pub(crate) async fn navigate_page(
        &self,
        request: styrene_ipc::types::PageNavigationRequest,
    ) -> Result<PageResponse, String> {
        let page = self.client.navigate_page(&request).await.map_err(describe)?;
        Ok(PageResponse { page })
    }

    pub(crate) async fn close_page(&self, session_id: &str) -> Result<(), String> {
        self.client.close_page(session_id).await.map_err(describe)
    }

    pub(crate) async fn start_file_download(
        &self,
        request: styrene_ipc::types::FileDownloadRequest,
    ) -> Result<styrene_ipc::types::FileDownloadInfo, String> {
        self.client.start_file_download(&request).await.map_err(describe)
    }

    pub(crate) async fn file_download(
        &self,
        download_id: &str,
    ) -> Result<styrene_ipc::types::FileDownloadInfo, String> {
        self.client.file_download(download_id).await.map_err(describe)
    }

    pub(crate) async fn cancel_file_download(
        &self,
        download_id: &str,
    ) -> Result<styrene_ipc::types::FileDownloadInfo, String> {
        self.client.cancel_file_download(download_id).await.map_err(describe)
    }

    pub(crate) async fn save_file_download(
        &self,
        download_id: &str,
        destination: &str,
    ) -> Result<styrene_ipc::types::FileDownloadInfo, String> {
        self.client.save_file_download(download_id, destination).await.map_err(describe)
    }

    pub(crate) async fn path_table(&self) -> Result<Vec<PathTableEntry>, String> {
        self.client.path_table().await.map(path_table_entries).map_err(describe)
    }

    pub(crate) async fn interface_stats(&self) -> Result<Vec<InterfaceStats>, String> {
        self.client.interface_stats().await.map_err(describe)
    }

    pub(crate) async fn links(&self) -> Result<LinkSnapshot, String> {
        self.client.links().await.map_err(describe)
    }

    pub(crate) async fn query_conversations(&self) -> Result<Vec<ConversationInfo>, String> {
        self.client.conversations().await.map_err(describe)
    }

    pub(crate) async fn query_conversation_page(
        &self,
        cursor: Option<&str>,
    ) -> Result<crate::backend::BackendPage<ConversationInfo>, String> {
        let paged = self.client.conversation_page(cursor, 50).await.map_err(describe)?;
        Ok(crate::backend::BackendPage {
            items: paged.page.conversations,
            next_cursor: paged.page.next_cursor,
            pagination_supported: paged.pagination_supported,
            reset: paged.reset,
        })
    }

    pub(crate) async fn query_messages(
        &self,
        peer_hash: &str,
        limit: u32,
    ) -> Result<Vec<MessageInfo>, String> {
        self.client.messages(peer_hash, limit).await.map_err(describe)
    }

    pub(crate) async fn query_message(
        &self,
        message_id: &str,
    ) -> Result<Option<MessageInfo>, String> {
        self.client.message(message_id).await.map_err(describe)
    }

    pub(crate) async fn query_message_page(
        &self,
        peer_hash: &str,
        cursor: Option<&str>,
    ) -> Result<crate::backend::BackendPage<MessageInfo>, String> {
        let paged = self.client.message_page(peer_hash, cursor, 50).await.map_err(describe)?;
        Ok(crate::backend::BackendPage {
            items: paged.page.messages,
            next_cursor: paged.page.next_cursor,
            pagination_supported: paged.pagination_supported,
            reset: paged.reset,
        })
    }

    pub(crate) async fn propagation_snapshot(
        &self,
        cursor: Option<&str>,
    ) -> Result<PropagationSnapshot, String> {
        let mut query = PropagationQuery::default();
        query.limit = 100;
        query.cursor = cursor.map(ToOwned::to_owned);
        self.client.propagation_snapshot(&query).await.map_err(describe)
    }

    pub(crate) async fn standard_propagation_snapshot(
        &self,
    ) -> Result<StandardPropagationSnapshot, String> {
        self.client.standard_propagation().await.map_err(describe)
    }

    pub(crate) async fn device_status(
        &self,
        destination: &str,
    ) -> Result<RemoteStatusInfo, String> {
        self.client.device_status(destination, 30).await.map_err(describe)
    }

    pub(crate) async fn exec(
        &self,
        destination: &str,
        command: &str,
        args: &[String],
    ) -> Result<ExecResult, String> {
        self.client.exec(destination, command, args, 60).await.map_err(describe)
    }

    pub(crate) async fn reboot(
        &self,
        destination: &str,
        delay: Option<u64>,
    ) -> Result<RebootResult, String> {
        self.client.reboot_device(destination, delay.unwrap_or(0)).await.map_err(describe)
    }

    pub(crate) async fn fleet_apply(
        &self,
        destination: &str,
        profile_base64: &str,
    ) -> Result<styrene_ipc::types::ConfigApplyResult, String> {
        use base64::Engine;
        let profile = base64::engine::general_purpose::STANDARD
            .decode(profile_base64)
            .map_err(|error| format!("fleet profile is not valid base64: {error}"))?;
        self.client.fleet_apply(destination, &profile, true, 60).await.map_err(describe)
    }

    pub(crate) async fn block_peer(&self, identity_hash: &str) -> Result<(), String> {
        self.client.block_peer(identity_hash).await.map_err(describe)
    }

    pub(crate) async fn start_network_operation(
        &self,
        request: StartNetworkOperationInfo,
    ) -> Result<NetworkOperationInfo, String> {
        self.client.start_network_operation(&request).await.map_err(describe)
    }

    pub(crate) async fn cancel_network_operation(
        &self,
        operation_id: &str,
    ) -> Result<NetworkOperationInfo, String> {
        self.client.cancel_network_operation(operation_id).await.map_err(describe)
    }

    pub(crate) async fn network_operations(&self) -> Result<Vec<NetworkOperationInfo>, String> {
        self.client.network_operations().await.map_err(describe)
    }

    pub(crate) async fn start_request(
        &self,
        request: StartRequestInfo,
    ) -> Result<RequestObservationInfo, String> {
        self.client.start_request(&request).await.map_err(describe)
    }

    pub(crate) async fn cancel_request(
        &self,
        request_id: &str,
    ) -> Result<RequestObservationInfo, String> {
        self.client.cancel_request(request_id).await.map_err(describe)
    }

    pub(crate) async fn requests(&self) -> Result<Vec<RequestObservationInfo>, String> {
        self.client.requests().await.map_err(describe)
    }

    pub(crate) async fn resources(&self) -> Result<Vec<ResourceTransferInfo>, String> {
        self.client.resources().await.map_err(describe)
    }

    pub(crate) async fn cancel_resource(&self, resource_hash: &str) -> Result<bool, String> {
        self.client.cancel_resource(resource_hash).await.map_err(describe)
    }
}

/// Open a Live command session and a Live event session against `socket_path`.
pub(crate) async fn connect_ipc(
    socket_path: &Path,
    generation: crate::backend::ConnectionGeneration,
) -> Result<(RequestBroker, mpsc::Receiver<DaemonEvent>), String> {
    let command = Session::live(socket_path).await.map_err(|error| error.to_string())?;
    attach(command, generation).await
}

/// Start an Embedded runtime session and attach a Live event session to its
/// private endpoint. Nothing is left running when this fails.
pub(crate) async fn connect_embedded(
    ephemeral: bool,
    generation: crate::backend::ConnectionGeneration,
) -> Result<(RequestBroker, mpsc::Receiver<DaemonEvent>, EmbeddedDaemon), String> {
    info!(target: "dx::bridge", "starting embedded daemon");
    let command =
        Session::embedded(EmbeddedConfig { db: None, config: None, identity: None, ephemeral })
            .await
            .map_err(|error| format!("embedded daemon boot failed: {error}"))?;
    debug_assert_eq!(command.profile(), SessionProfile::Quick);
    let (broker, events) = attach(command, generation).await?;
    info!(target: "dx::bridge", "embedded daemon fully initialized");
    let runtime = EmbeddedDaemon { session: broker.command_session.clone() };
    Ok((broker, events, runtime))
}

/// Open a persistent managed Local profile at `root` (creating it when it
/// does not exist yet) and attach a Live event session to its endpoint.
pub(crate) async fn connect_local(
    root: &Path,
    generation: crate::backend::ConnectionGeneration,
) -> Result<(RequestBroker, mpsc::Receiver<DaemonEvent>, EmbeddedDaemon), String> {
    // Unix socket paths are length-limited; keep the host-private runtime
    // parent short and off the profile root.
    let runtime_parent = std::env::temp_dir().join("styrene-rt");
    std::fs::create_dir_all(&runtime_parent)
        .map_err(|error| format!("create runtime parent {}: {error}", runtime_parent.display()))?;
    let command = Session::managed(ManagedTarget::Local {
        root: root.to_path_buf(),
        runtime_parent,
        display_name: Some("Desktop local profile"),
    })
    .await
    .map_err(|error| format!("local profile failed: {error}"))?;
    debug_assert_eq!(command.profile(), SessionProfile::Local);
    let (broker, events) = attach(command, generation).await?;
    let runtime = EmbeddedDaemon { session: broker.command_session.clone() };
    Ok((broker, events, runtime))
}

/// Build the broker, the initial snapshot events, the event reader, and the
/// poller around a negotiated command session.
async fn attach(
    command: Session,
    generation: crate::backend::ConnectionGeneration,
) -> Result<(RequestBroker, mpsc::Receiver<DaemonEvent>), String> {
    let endpoint = command.metadata().endpoint.clone();
    let client = command.client().clone();
    let identity = client.identity().await.ok();
    let status = Some(command.metadata().status.clone());
    let devices = client.devices(false).await.unwrap_or_default();
    let paths = client.path_table().await.ok().map(path_table_entries);
    // Connected and EventGeneration are also sent before the receiver is returned.
    let initial_event_count = 2
        + usize::from(identity.is_some())
        + usize::from(status.is_some())
        + devices.len()
        + usize::from(paths.is_some());
    let (tx, rx) = mpsc::channel(event_channel_capacity(initial_event_count + 1));
    let _ = tx.send(DaemonEvent::Connected).await;
    if let Some(profile) = command.profile_info() {
        let _ = tx.send(DaemonEvent::Profile(Box::new(profile.clone()))).await;
    }
    if let Some(info) = identity {
        let _ = tx.send(DaemonEvent::Identity(info)).await;
    }
    if let Some(status) = status {
        let _ = tx.send(DaemonEvent::Status(status)).await;
    }
    for device in devices {
        let _ = tx.send(DaemonEvent::PeerDiscovered(device)).await;
    }
    if let Some(paths) = paths {
        let _ = tx.send(DaemonEvent::PathTable(paths)).await;
    }

    // A dedicated subscription session keeps pushed events off the command
    // connection. It is a Live session even when the command session is
    // Embedded: the runtime is owned once, by the command session.
    let command_session = Arc::new(Mutex::new(Some(command)));
    let events_session = match Session::live(&endpoint).await {
        Ok(session) => session,
        Err(error) => {
            close_session(&command_session).await;
            return Err(format!("event session: {error}"));
        }
    };
    let event_generation = events_session.daemon_generation();
    let _ = tx.send(DaemonEvent::EventGeneration(event_generation)).await;
    // Take the receiver before subscribing so no pushed event is missed.
    let frames = events_session.events();
    if let Err(error) = events_session
        .subscribe(&[
            EventTopic::Devices,
            EventTopic::Messages,
            EventTopic::Links,
            EventTopic::Routes,
            EventTopic::NetworkOperations,
            EventTopic::Requests,
            EventTopic::Resources,
        ])
        .await
    {
        close_session(&command_session).await;
        return Err(format!("event subscription: {error}"));
    }
    let event_session = Arc::new(Mutex::new(Some(events_session)));
    let broker = RequestBroker::new(client, command_session, event_session, generation);
    tokio::spawn(event_reader(
        frames,
        tx.clone(),
        broker.dropped_updates.clone(),
        event_generation,
    ));
    spawn_poller(broker.clone(), tx);
    Ok((broker, rx))
}

fn event_channel_capacity(initial_event_count: usize) -> usize {
    512.max(initial_event_count)
}

async fn event_reader(
    mut frames: broadcast::Receiver<EventFrame>,
    tx: mpsc::Sender<DaemonEvent>,
    dropped_updates: Arc<AtomicU64>,
    event_generation: u64,
) {
    info!(target: "dx::reader", "event reader started");
    loop {
        match frames.recv().await {
            Ok(frame) => {
                debug!(target: "dx::reader", msg_type = ?frame.message_type, "received frame");
                let Some(event) = frame_to_event(&frame) else { continue };
                match tx.try_send(event) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        dropped_updates.fetch_add(1, Ordering::Relaxed);
                        warn!(target: "dx::reader", "event channel full, dropping update");
                        let _ = tx
                            .send(DaemonEvent::ReconcileRequired {
                                dropped: 1,
                                connection_generation: event_generation,
                            })
                            .await;
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        warn!(target: "dx::reader", "channel closed, stopping");
                        break;
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(dropped)) => {
                dropped_updates.fetch_add(dropped, Ordering::Relaxed);
                warn!(target: "dx::reader", dropped, "event fanout lagged, reconcile required");
                let _ = tx
                    .send(DaemonEvent::ReconcileRequired {
                        dropped,
                        connection_generation: event_generation,
                    })
                    .await;
            }
            Err(broadcast::error::RecvError::Closed) => {
                let _ = tx.send(DaemonEvent::Disconnected("event connection closed".into())).await;
                break;
            }
        }
    }
}

fn spawn_poller(broker: RequestBroker, tx: mpsc::Sender<DaemonEvent>) {
    tokio::spawn(
        async move {
            info!(target: "dx::poller", "poller started");
            let dropped = broker.dropped_updates.clone();
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                let start = std::time::Instant::now();
                match broker.client.status().await {
                    Ok(status) => send_polled_event(&tx, DaemonEvent::Status(status), &dropped),
                    Err(error) => {
                        let _ = tx.send(DaemonEvent::Disconnected(error.to_string())).await;
                        break;
                    }
                }
                if let Ok(devices) = broker.client.devices(false).await {
                    debug!(target: "dx::poller", count = devices.len(), "polled devices");
                    for device in devices {
                        send_polled_event(&tx, DaemonEvent::PeerDiscovered(device), &dropped);
                    }
                }
                if let Ok(pages) = broker.local_page_inventory().await {
                    send_polled_event(&tx, DaemonEvent::LocalPageInventory(pages), &dropped);
                }
                match broker.path_table().await {
                    Ok(paths) => {
                        info!(target: "dx::poller", count = paths.len(), "polled path table");
                        send_polled_event(&tx, DaemonEvent::PathTable(paths), &dropped);
                    }
                    Err(error) => warn!(target: "dx::poller", %error, "path table poll failed"),
                }
                if let Ok(operations) = broker.network_operations().await {
                    for operation in operations {
                        send_polled_event(&tx, DaemonEvent::NetworkOperation(operation), &dropped);
                    }
                }
                if let Ok(requests) = broker.requests().await {
                    for request in requests {
                        send_polled_event(&tx, DaemonEvent::Request(request), &dropped);
                    }
                }
                if let Ok(resources) = broker.resources().await {
                    for resource in resources {
                        send_polled_event(&tx, DaemonEvent::Resource(resource), &dropped);
                    }
                }
                if let Ok(links) = broker.links().await {
                    for link in links.active.into_iter().chain(links.history) {
                        send_polled_event(&tx, DaemonEvent::LinkObservation(link), &dropped);
                    }
                }
                let elapsed_ms = start.elapsed().as_millis();
                debug!(target: "dx::poller", elapsed_ms, "tick complete");
            }
        }
        .instrument(info_span!("poller")),
    );
}

fn send_polled_event(tx: &mpsc::Sender<DaemonEvent>, event: DaemonEvent, dropped: &AtomicU64) {
    if tx.try_send(event).is_err() {
        dropped.fetch_add(1, Ordering::Relaxed);
    }
}

fn generation_fields(payload: &HashMap<String, MpValue>) -> (u64, u64) {
    (
        payload.get("dropped").and_then(MpValue::as_u64).unwrap_or(0),
        payload.get("connection_generation").and_then(MpValue::as_u64).unwrap_or(0),
    )
}

/// Map a pushed frame onto a desktop event. Frames that do not decode as
/// their canonical record are dropped rather than guessed at.
fn frame_to_event(frame: &EventFrame) -> Option<DaemonEvent> {
    match frame.message_type {
        MessageType::EventDevice => {
            let device: DeviceInfo = frame.typed().ok()?;
            (!device.destination_hash.is_empty()).then_some(DaemonEvent::PeerDiscovered(device))
        }
        MessageType::EventMessage => {
            let message: MessageInfo = frame.typed().ok()?;
            (!message.id.is_empty()).then(|| DaemonEvent::MessageReceived(Box::new(message)))
        }
        MessageType::EventLink => {
            let link: LinkEvent = frame.typed().ok()?;
            (!link.link_id.is_empty()).then_some(DaemonEvent::LinkObservation(link))
        }
        MessageType::EventRoute => frame.route_event().ok().map(DaemonEvent::RouteLifecycle),
        MessageType::EventNetworkOperation => {
            let operation: NetworkOperationInfo = frame.typed().ok()?;
            (!operation.operation_id.is_empty()).then_some(DaemonEvent::NetworkOperation(operation))
        }
        MessageType::EventRequest if frame.text("kind") == "reconcile_required" => {
            let (dropped, connection_generation) = generation_fields(&frame.payload);
            Some(DaemonEvent::ReconcileRequests { dropped, connection_generation })
        }
        MessageType::EventRequest => frame.typed().ok().map(DaemonEvent::Request),
        MessageType::EventResource => frame.typed().ok().map(DaemonEvent::Resource),
        MessageType::EventReconcileRequired => {
            let (dropped, connection_generation) = generation_fields(&frame.payload);
            Some(DaemonEvent::ReconcileRequired { dropped, connection_generation })
        }
        MessageType::EventStandardPropagationChanged => {
            let (_, connection_generation) = generation_fields(&frame.payload);
            Some(DaemonEvent::StandardPropagationChanged { connection_generation })
        }
        MessageType::EventMessagingOperation => frame
            .payload
            .get("outcome")
            .cloned()
            .and_then(|value| styrene_ipc_client::decode_value(value, "outcome").ok())
            .map(Box::new)
            .map(DaemonEvent::MessagingOperation),
        _ => None,
    }
}

/// Project canonical path records onto the desktop's routing rows.
fn path_table_entries(paths: Vec<PathInfo>) -> Vec<PathTableEntry> {
    paths
        .into_iter()
        .filter(|path| !path.destination_hash.is_empty())
        .map(|path| PathTableEntry {
            destination_hash: path.destination_hash,
            hops: path.hops.map_or(0, |hops| u8::try_from(hops).unwrap_or(u8::MAX)),
            next_hop: path.next_hop.unwrap_or_default(),
            interface: path.interface.unwrap_or_default(),
            expires: path.expires,
            observation: path.observation,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use styrene_ipc::types::{ObservationSource, RouteEventKind, RouteLossReason};
    use styrene_ipc_client::ConnectionGeneration;

    fn event(message_type: MessageType, payload: HashMap<String, MpValue>) -> EventFrame {
        EventFrame { message_type, payload, generation: ConnectionGeneration(1) }
    }

    #[test]
    fn startup_channel_holds_high_cardinality_snapshot_before_consumer_starts() {
        assert_eq!(event_channel_capacity(4), 512);
        // Connected, identity, status, 1,425 peers, paths, and event generation.
        assert_eq!(event_channel_capacity(1_430), 1_430);
    }

    #[test]
    fn status_event_uses_authoritative_message_status_not_event_kind() {
        for status in ["sending", "cancelled", "failed: no route"] {
            let mut payload = HashMap::new();
            payload.insert("kind".into(), MpValue::from("status_changed"));
            payload.insert("id".into(), MpValue::from("message"));
            payload.insert("status".into(), MpValue::from(status));
            payload.insert("authentication_state".into(), MpValue::from("verified"));
            let event = frame_to_event(&event(MessageType::EventMessage, payload));
            assert!(matches!(
                event,
                Some(DaemonEvent::MessageReceived(message))
                    if message.id == "message" && message.status == status
            ));
        }
    }

    #[test]
    fn route_event_parser_preserves_loss_and_generation_metadata() {
        let mut payload = HashMap::new();
        payload.insert("kind".into(), MpValue::from("lost"));
        payload.insert("destination_hash".into(), MpValue::from("peer"));
        payload.insert("loss_reason".into(), MpValue::from("expired"));
        payload.insert("expires".into(), MpValue::from(700_i64));
        payload.insert("source".into(), MpValue::from("transport_path_table"));
        payload.insert("connection_generation".into(), MpValue::from(9_u64));
        payload.insert("route_age_secs".into(), MpValue::from(601_u64));
        payload.insert("route_connection_generation".into(), MpValue::from(9_u64));
        payload.insert("route_freshness_threshold_secs".into(), MpValue::from(300_u64));
        payload.insert("route_stale".into(), MpValue::Boolean(true));

        match frame_to_event(&event(MessageType::EventRoute, payload)) {
            Some(DaemonEvent::RouteLifecycle(event)) => {
                assert_eq!(event.kind, RouteEventKind::Lost);
                assert_eq!(event.loss_reason, Some(RouteLossReason::Expired));
                assert_eq!(event.route.expires, Some(700));
                assert_eq!(event.route.observation.age_secs, Some(601));
                assert_eq!(event.route.observation.connection_generation, Some(9));
                assert_eq!(event.route.observation.freshness_threshold_secs, Some(300));
                assert_eq!(event.observation.connection_generation, Some(9));
            }
            other => panic!("expected route event, got {other:?}"),
        }
    }

    #[test]
    fn network_operation_events_keep_the_coordinator_source_and_correlation() {
        let mut payload = HashMap::new();
        payload.insert("operation_id".into(), MpValue::from("op-1"));
        payload.insert("kind".into(), MpValue::from("announce"));
        payload.insert("progress".into(), MpValue::from("dispatched"));
        payload.insert("source".into(), MpValue::from("operation_coordinator"));
        payload.insert("correlation_id".into(), MpValue::from("corr-1"));
        payload.insert("connection_generation".into(), MpValue::from(4_u64));
        match frame_to_event(&event(MessageType::EventNetworkOperation, payload)) {
            Some(DaemonEvent::NetworkOperation(operation)) => {
                assert_eq!(operation.operation_id, "op-1");
                assert_eq!(operation.observation.source, ObservationSource::OperationCoordinator);
                assert_eq!(operation.observation.correlation_id.as_deref(), Some("corr-1"));
                assert_eq!(operation.observation.connection_generation, Some(4));
            }
            other => panic!("expected network operation, got {other:?}"),
        }
    }

    #[test]
    fn unknown_capabilities_and_reconcile_events_decode() {
        let mut payload = HashMap::new();
        payload.insert("destination_hash".into(), MpValue::from("peer"));
        payload.insert(
            "discovered_capabilities".into(),
            MpValue::Array(vec![
                MpValue::from("native_nomadnet_host"),
                MpValue::from("future_capability"),
            ]),
        );
        assert!(matches!(
            frame_to_event(&event(MessageType::EventDevice, payload)),
            Some(DaemonEvent::PeerDiscovered(device)) if device.destination_hash == "peer"
        ));
        let mut payload = HashMap::new();
        payload.insert("kind".into(), MpValue::from("reconcile_required"));
        payload.insert("dropped".into(), MpValue::from(3_u64));
        payload.insert("connection_generation".into(), MpValue::from(8_u64));
        assert!(matches!(
            frame_to_event(&event(MessageType::EventRequest, payload)),
            Some(DaemonEvent::ReconcileRequests { dropped: 3, connection_generation: 8 })
        ));
    }

    #[test]
    fn path_rows_project_canonical_routes_and_defaults() {
        let mut path = PathInfo::default();
        path.destination_hash = "peer".into();
        path.hops = Some(300);
        path.expires = Some(9);
        path.observation.source = ObservationSource::TransportPathTable;
        let mut empty = PathInfo::default();
        empty.hops = Some(1);
        let rows = path_table_entries(vec![path, empty]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].destination_hash, "peer");
        assert_eq!(rows[0].hops, u8::MAX);
        assert_eq!(rows[0].next_hop, "");
        assert_eq!(rows[0].expires, Some(9));
        assert_eq!(rows[0].observation.source, ObservationSource::TransportPathTable);
    }

    #[test]
    fn full_event_channel_records_dropped_update() {
        let (tx, _events) = mpsc::channel(1);
        tx.try_send(DaemonEvent::Connected).expect("seed event channel");
        let dropped = AtomicU64::new(0);
        send_polled_event(&tx, DaemonEvent::Connected, &dropped);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn embedded_runtime_owns_and_removes_ephemeral_state() {
        let (broker, events, runtime) = tokio::time::timeout(
            Duration::from_secs(30),
            connect_embedded(true, crate::backend::ConnectionGeneration(1)),
        )
        .await
        .expect("embedded startup timed out")
        .expect("embedded startup failed");
        let endpoint = runtime.endpoint().await.expect("embedded endpoint");
        assert!(endpoint.exists());
        assert!(broker.path_table().await.is_ok());
        drop(events);
        broker.shutdown().await;
        runtime.shutdown().await;
        assert!(!endpoint.exists());
        assert!(broker.path_table().await.is_err());
    }

    #[test]
    fn every_backend_command_has_an_explicit_execution_capability_policy() {
        let commands = vec![
            DaemonCommand::SendChat {
                peer_hash: String::new(),
                content: String::new(),
                title: None,
                delivery_method: "direct".into(),
                attachments: Vec::new(),
            },
            DaemonCommand::LoadDraft { peer_hash: String::new() },
            DaemonCommand::SaveDraft { peer_hash: String::new(), content: String::new() },
            DaemonCommand::DiscardDraft { peer_hash: String::new() },
            DaemonCommand::RetryMessage { message_id: String::new() },
            DaemonCommand::CancelMessage { message_id: String::new() },
            DaemonCommand::BrowsePage { address: crate::state::PageAddress::local_index() },
            DaemonCommand::NavigatePage(Default::default()),
            DaemonCommand::ClosePage { session_id: String::new() },
            DaemonCommand::StartFileDownload(Default::default()),
            DaemonCommand::QueryFileDownload { download_id: String::new() },
            DaemonCommand::CancelFileDownload { download_id: String::new() },
            DaemonCommand::SaveFileDownload {
                download_id: String::new(),
                destination: String::new(),
            },
            DaemonCommand::RefreshPathTable,
            DaemonCommand::RefreshInterfaces,
            DaemonCommand::RefreshLinks,
            DaemonCommand::RefreshOperations,
            DaemonCommand::RefreshRequests,
            DaemonCommand::RefreshResources,
            DaemonCommand::StartNetworkOperation(Default::default()),
            DaemonCommand::CancelNetworkOperation { operation_id: String::new() },
            DaemonCommand::StartRequest(Default::default()),
            DaemonCommand::CancelRequest { request_id: String::new() },
            DaemonCommand::CancelResource { resource_hash: String::new() },
            DaemonCommand::RefreshPropagation { cursor: None },
            DaemonCommand::LoadConversations { cursor: None },
            DaemonCommand::LoadMessages { peer_hash: String::new(), cursor: None },
            DaemonCommand::QueryMessage { message_id: String::new() },
            DaemonCommand::FleetStatus { destination: String::new() },
            DaemonCommand::FleetExec {
                destination: String::new(),
                command: String::new(),
                args: Vec::new(),
            },
            DaemonCommand::FleetReboot { destination: String::new(), delay: None },
            DaemonCommand::FleetApply { destination: String::new(), profile_base64: String::new() },
            DaemonCommand::BlockPeer { identity_hash: String::new() },
        ];

        assert_eq!(commands.len(), 33);
        assert!(commands.iter().enumerate().all(|(index, command)| {
            command.required_capability().is_some() || matches!(index, 19 | 20)
        }));
        assert_eq!(commands[6].required_capability(), Some("page.browse"));
        assert_eq!(
            commands
                .iter()
                .find(|command| matches!(command, DaemonCommand::FleetReboot { .. }))
                .and_then(DaemonCommand::required_capability),
            Some("rpc.reboot")
        );
    }
}
