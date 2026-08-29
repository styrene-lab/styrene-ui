use std::future::pending;
use std::time::Duration;

use async_channel::{Receiver, Sender};
use styrene_ui_apple_bridge::{
    CoreBluetoothFailure, CoreBluetoothGeneration, IosBleAdapter, IosBleEvent, IosBleEventStream,
    NativeAuthorization, approved_ble_peripheral, bluetooth_authorization,
    store_approved_ble_peripheral,
};
use styrene_ui_platform::{
    AuthorizationState, BleAdapterState, BleApprovedPeripheral, BleControlFailure, BleControlPhase,
    BleControlState, BlePeripheralId, BleRNodeByteAttempt, PermissionKind,
};

use crate::ble_session::run_mobile_ble_rnode;
use crate::session::MobileSession;

const COMMAND_CAPACITY: usize = 8;
const CANDIDATE_CAPACITY: usize = 24;
const SCAN_DEADLINE: Duration = Duration::from_secs(10);
const CONNECTION_DEADLINE: Duration = Duration::from_secs(15);

#[derive(Clone, Copy)]
enum PendingOperation {
    Scan,
    Connect,
}

#[derive(Clone)]
pub struct IosBleHost {
    commands: Sender<IosBleCommand>,
    command_receiver: Receiver<IosBleCommand>,
    updates: Sender<BleControlState>,
    update_receiver: Receiver<BleControlState>,
}

#[derive(Clone)]
enum IosBleCommand {
    Scan,
    Select(BlePeripheralId),
    Retry,
    Forget,
}

impl IosBleHost {
    pub fn new() -> Self {
        let (commands, command_receiver) = async_channel::bounded(COMMAND_CAPACITY);
        let (updates, update_receiver) = async_channel::bounded(1);
        Self { commands, command_receiver, updates, update_receiver }
    }

    pub fn scan(&self) {
        let _ = self.commands.try_send(IosBleCommand::Scan);
    }

    pub fn select(&self, id: BlePeripheralId) {
        let _ = self.commands.try_send(IosBleCommand::Select(id));
    }

    pub fn retry(&self) {
        let _ = self.commands.try_send(IosBleCommand::Retry);
    }

    pub fn forget(&self) {
        let _ = self.commands.try_send(IosBleCommand::Forget);
    }

    pub async fn next_update(&self) -> Option<BleControlState> {
        self.update_receiver.recv().await.ok()
    }

    #[allow(clippy::too_many_lines)]
    pub async fn run(&self, session: MobileSession) {
        let Ok(node) = session.backend_node().await else {
            self.publish(BleControlState::default());
            return;
        };
        let approved = approved_ble_peripheral().map(|id| BleApprovedPeripheral { id });
        let mut state = BleControlState {
            permission: authorization(bluetooth_authorization()),
            adapter: BleAdapterState::Unavailable,
            phase: BleControlPhase::Idle,
            candidates: Vec::new(),
            approved,
            failure: None,
        };
        let mut generation = 0_u64;
        let mut adapter = None::<IosBleAdapter>;
        let mut events = None::<IosBleEventStream>;
        let mut pending_scan = false;
        let mut pending_connect = state.approved.as_ref().map(|approved| approved.id.clone());
        let mut deadline = None;
        if pending_connect.is_some() && state.permission == AuthorizationState::Granted {
            install_adapter(&mut adapter, &mut events);
            state.phase = BleControlPhase::Reconnecting;
            deadline = Some((
                tokio::time::Instant::now() + CONNECTION_DEADLINE,
                PendingOperation::Connect,
            ));
        }
        self.publish(state.clone());

        loop {
            let event = async {
                match events.as_ref() {
                    Some(events) => events.next().await,
                    None => pending().await,
                }
            };
            let operation_timeout = async {
                match deadline {
                    Some((deadline, operation)) => {
                        tokio::time::sleep_until(deadline).await;
                        operation
                    }
                    None => pending().await,
                }
            };
            tokio::select! {
                command = self.command_receiver.recv() => {
                    let Ok(command) = command else { return };
                    match command {
                        IosBleCommand::Scan => {
                            let permission = crate::platform::request_permission(PermissionKind::Bluetooth).await;
                            state.permission = permission.map_or(
                                AuthorizationState::Unavailable,
                                |permission| permission.state,
                            );
                            state.failure = None;
                            state.candidates.clear();
                            if state.permission == AuthorizationState::Granted {
                                if let Some(mut current) = adapter.take() {
                                    current.close();
                                }
                                install_adapter(&mut adapter, &mut events);
                                pending_scan = true;
                                pending_connect = None;
                                state.phase = BleControlPhase::Scanning;
                                deadline = Some((tokio::time::Instant::now() + SCAN_DEADLINE, PendingOperation::Scan));
                            } else {
                                state.phase = BleControlPhase::Idle;
                            }
                            self.publish(state.clone());
                        }
                        IosBleCommand::Select(id) => {
                            if !state.candidates.iter().any(|candidate| candidate.id == id) {
                                continue;
                            }
                            if let Some(adapter) = adapter.as_ref() {
                                generation = next_generation(generation);
                                let current = CoreBluetoothGeneration::new(generation);
                                state.approved = Some(BleApprovedPeripheral { id: id.clone() });
                                store_approved_ble_peripheral(Some(&id));
                                node.platform_service().set_bluetooth_approved(true).await;
                                state.phase = BleControlPhase::Connecting;
                                state.failure = None;
                                deadline = Some((tokio::time::Instant::now() + CONNECTION_DEADLINE, PendingOperation::Connect));
                                if let Err(error) = adapter.connect(current, &id) {
                                    deadline = None;
                                    state.phase = BleControlPhase::Idle;
                                    state.failure = Some(BleControlFailure::ConnectionFailed);
                                    let _ = error;
                                }
                                self.publish(state.clone());
                            }
                        }
                        IosBleCommand::Retry => {
                            let Some(id) = state.approved.as_ref().map(|approved| approved.id.clone()) else {
                                continue;
                            };
                            if let Some(mut current) = adapter.take() {
                                current.close();
                            }
                            install_adapter(&mut adapter, &mut events);
                            pending_connect = Some(id);
                            pending_scan = false;
                            state.phase = BleControlPhase::Reconnecting;
                            state.failure = None;
                            deadline = Some((tokio::time::Instant::now() + CONNECTION_DEADLINE, PendingOperation::Connect));
                            self.publish(state.clone());
                        }
                        IosBleCommand::Forget => {
                            if let Some(mut current) = adapter.take() {
                                current.close();
                            }
                            events = None;
                            pending_connect = None;
                            pending_scan = false;
                            deadline = None;
                            store_approved_ble_peripheral(None);
                            node.platform_service().set_bluetooth_approved(false).await;
                            state.approved = None;
                            state.phase = BleControlPhase::Idle;
                            state.failure = None;
                            self.publish(state.clone());
                        }
                    }
                }
                event = event => {
                    let Some(event) = event else {
                        adapter = None;
                        events = None;
                        continue;
                    };
                    match event {
                        IosBleEvent::AdapterChanged(adapter_state) => {
                            state.adapter = adapter_state;
                            if adapter_state == BleAdapterState::Ready {
                                if pending_scan {
                                    generation = next_generation(generation);
                                    if let Some(adapter) = adapter.as_ref()
                                        && adapter.start_scan(CoreBluetoothGeneration::new(generation)).is_ok()
                                    {
                                        pending_scan = false;
                                    }
                                } else if let Some(id) = pending_connect.take() {
                                    generation = next_generation(generation);
                                    node.platform_service().set_bluetooth_approved(true).await;
                                    if let Some(adapter) = adapter.as_ref()
                                        && adapter.connect(CoreBluetoothGeneration::new(generation), &id).is_err()
                                    {
                                        deadline = None;
                                        state.phase = BleControlPhase::Idle;
                                        state.failure = Some(BleControlFailure::ConnectionFailed);
                                    }
                                }
                            }
                            self.publish(state.clone());
                        }
                        IosBleEvent::Candidate { candidate, .. } => {
                            if let Some(existing) = state
                                .candidates
                                .iter_mut()
                                .find(|existing| existing.id == candidate.id)
                            {
                                *existing = candidate;
                            } else if state.candidates.len() < CANDIDATE_CAPACITY {
                                state.candidates.push(candidate);
                            }
                            self.publish(state.clone());
                        }
                        IosBleEvent::Ready { write_limit, .. } => {
                            deadline = None;
                            state.phase = BleControlPhase::Connecting;
                            self.publish(state.clone());
                            if let Some(adapter) = adapter.as_mut() {
                                let forgotten = run_connected(
                                    &node,
                                    adapter,
                                    write_limit,
                                    &self.command_receiver,
                                    &mut state,
                                    &self.updates,
                                )
                                .await;
                                if forgotten {
                                    store_approved_ble_peripheral(None);
                                    node.platform_service().set_bluetooth_approved(false).await;
                                }
                            }
                            self.publish(state.clone());
                        }
                        IosBleEvent::Failed { failure, .. } => {
                            deadline = None;
                            state.phase = BleControlPhase::Idle;
                            state.failure = Some(control_failure(failure));
                            self.publish(state.clone());
                        }
                        IosBleEvent::Disconnected { .. } => {
                            deadline = None;
                            if state.approved.is_some() {
                                state.phase = BleControlPhase::Idle;
                                state.failure = Some(BleControlFailure::ConnectionInterrupted);
                            }
                            self.publish(state.clone());
                        }
                    }
                }
                operation = operation_timeout => {
                    deadline = None;
                    pending_scan = false;
                    pending_connect = None;
                    match operation {
                        PendingOperation::Scan => {
                            if let Some(adapter) = adapter.as_ref() {
                                adapter.stop_scan();
                            }
                            state.phase = BleControlPhase::Idle;
                        }
                        PendingOperation::Connect => {
                            if let Some(adapter) = adapter.as_mut() {
                                adapter.close();
                            }
                            state.phase = BleControlPhase::Idle;
                            state.failure = Some(BleControlFailure::ConnectionFailed);
                        }
                    }
                    self.publish(state.clone());
                }
            }
        }
    }

    fn publish(&self, state: BleControlState) {
        let _ = self.updates.force_send(state);
    }
}

async fn run_connected(
    node: &styrened::mobile::MobileNode,
    adapter: &mut IosBleAdapter,
    write_limit: styrene_ui_platform::BleWriteLimit,
    commands: &Receiver<IosBleCommand>,
    state: &mut BleControlState,
    updates: &Sender<BleControlState>,
) -> bool {
    let (cancel, cancelled) = async_channel::bounded(1);
    let pump = run_mobile_ble_rnode(node, adapter, write_limit, &cancelled);
    tokio::pin!(pump);
    let mut forgotten = false;
    loop {
        tokio::select! {
            result = &mut pump => {
                state.phase = BleControlPhase::Idle;
                if !forgotten && result.is_err() {
                    state.failure = Some(BleControlFailure::ConnectionInterrupted);
                }
                return forgotten;
            }
            command = commands.recv() => {
                match command {
                    Ok(IosBleCommand::Forget) => {
                        forgotten = true;
                        state.approved = None;
                        state.failure = None;
                        let _ = updates.force_send(state.clone());
                        let _ = cancel.try_send(());
                    }
                    Ok(_) => {}
                    Err(_) => {
                        let _ = cancel.try_send(());
                    }
                }
            }
        }
    }
}

fn install_adapter(adapter: &mut Option<IosBleAdapter>, events: &mut Option<IosBleEventStream>) {
    let mut next = IosBleAdapter::new();
    *events = next.take_event_stream();
    *adapter = Some(next);
}

const fn authorization(state: NativeAuthorization) -> AuthorizationState {
    match state {
        NativeAuthorization::NotDetermined => AuthorizationState::NotDetermined,
        NativeAuthorization::Granted => AuthorizationState::Granted,
        NativeAuthorization::Denied => AuthorizationState::Denied,
        NativeAuthorization::Restricted => AuthorizationState::Restricted,
        NativeAuthorization::Unavailable => AuthorizationState::Unavailable,
    }
}

const fn next_generation(current: u64) -> u64 {
    match current.checked_add(1) {
        Some(next) => next,
        None => 1,
    }
}

const fn control_failure(failure: CoreBluetoothFailure) -> BleControlFailure {
    match failure {
        CoreBluetoothFailure::NusServiceMissing
        | CoreBluetoothFailure::WriteCharacteristicMissing
        | CoreBluetoothFailure::NotifyCharacteristicMissing
        | CoreBluetoothFailure::WriteWithResponseMissing
        | CoreBluetoothFailure::NotificationsUnsupported
        | CoreBluetoothFailure::InvalidWriteLimit => BleControlFailure::IncompatiblePeripheral,
        CoreBluetoothFailure::ConnectionFailed => BleControlFailure::ConnectionFailed,
        _ => BleControlFailure::ConnectionInterrupted,
    }
}
