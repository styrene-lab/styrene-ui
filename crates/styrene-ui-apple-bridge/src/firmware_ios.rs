use std::cell::RefCell;
use std::sync::Mutex;

use async_channel::{Receiver, Sender};
use futures_lite::future;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, DefinedClass, define_class, msg_send};
use objc2_core_bluetooth::{
    CBCentralManager, CBCentralManagerDelegate, CBCharacteristic, CBCharacteristicProperties,
    CBCharacteristicWriteType, CBManagerState, CBPeripheral, CBPeripheralDelegate, CBService,
    CBUUID,
};
use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSError, NSNumber, NSObject, NSObjectProtocol, NSString, NSUUID,
};
use styrene_ui_platform::{BlePeripheralId, PlatformFailure, PlatformFuture};

use crate::{
    CoreBluetoothDfuApply, CoreBluetoothDfuBoundary, CoreBluetoothDfuCharacteristics,
    CoreBluetoothDfuDisconnect, CoreBluetoothDfuEffect, CoreBluetoothDfuFailure,
    CoreBluetoothGeneration, CoreBluetoothManagerState, IosBleDfuHandoff,
    LEGACY_DFU_CONTROL_POINT_UUID, LEGACY_DFU_PACKET_UUID, LEGACY_DFU_SERVICE_UUID,
    LEGACY_DFU_VERSION_UUID, LegacyDfuGatt, LegacyDfuProgress, LegacyDfuRunFailure,
    RAK4631_PACKET_BYTES, Rak4631LegacyDfuPlan, run_rak4631_legacy_dfu,
};

const CHANNEL_CAPACITY: usize = 8;

struct DfuState {
    generation: CoreBluetoothGeneration,
    peripheral_id: String,
    active: bool,
    manager_ready: bool,
    boundary: CoreBluetoothDfuBoundary,
    characteristics: Option<CoreBluetoothDfuCharacteristics>,
}

struct DfuDelegateIvars {
    state: Mutex<DfuState>,
    manager_ready: Sender<Result<(), PlatformFailure>>,
    ready: Sender<Result<u16, PlatformFailure>>,
    notifications: Sender<Result<Vec<u8>, PlatformFailure>>,
    control_writes: Sender<Result<(), PlatformFailure>>,
    flow_ready: Sender<()>,
    disconnected: Sender<Result<(), PlatformFailure>>,
    failures: Sender<PlatformFailure>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[ivars = DfuDelegateIvars]
    struct DfuDelegate;

    unsafe impl NSObjectProtocol for DfuDelegate {}

    unsafe impl CBCentralManagerDelegate for DfuDelegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        unsafe fn central_manager_did_update_state(&self, central: &CBCentralManager) {
            match map_manager_state(unsafe { central.state() }) {
                CoreBluetoothManagerState::PoweredOn => {
                    if let Ok(mut state) = self.ivars().state.lock() {
                        state.manager_ready = true;
                    }
                    let _ = self.ivars().manager_ready.try_send(Ok(()));
                }
                CoreBluetoothManagerState::Unknown | CoreBluetoothManagerState::Resetting => {
                    if self.manager_was_ready() {
                        self.fail(failure("ios_dfu_adapter_reset", true));
                    }
                }
                CoreBluetoothManagerState::Unsupported
                | CoreBluetoothManagerState::Unauthorized
                | CoreBluetoothManagerState::PoweredOff => {
                    self.fail(failure("ios_dfu_adapter_unavailable", true));
                }
            }
        }

        #[unsafe(method(centralManager:didDiscoverPeripheral:advertisementData:RSSI:))]
        unsafe fn central_manager_did_discover(
            &self,
            _: &CBCentralManager,
            _: &CBPeripheral,
            _: &NSDictionary<NSString, objc2::runtime::AnyObject>,
            _: &NSNumber,
        ) {
            // The exact DFU identity is supplied by policy. Discovery never selects a peer.
        }

        #[unsafe(method(centralManager:didConnectPeripheral:))]
        unsafe fn central_manager_did_connect(
            &self,
            _: &CBCentralManager,
            peripheral: &CBPeripheral,
        ) {
            if !self.is_active(peripheral) {
                return;
            }
            unsafe { peripheral.setDelegate(Some(ProtocolObject::from_ref(self))) };
            let service = uuid(LEGACY_DFU_SERVICE_UUID);
            let services = NSArray::from_slice(&[&*service]);
            unsafe { peripheral.discoverServices(Some(&services)) };
        }

        #[unsafe(method(centralManager:didFailToConnectPeripheral:error:))]
        unsafe fn central_manager_did_fail_to_connect(
            &self,
            _: &CBCentralManager,
            peripheral: &CBPeripheral,
            _: Option<&NSError>,
        ) {
            if self.is_active(peripheral) {
                self.fail(failure("ios_dfu_connect_failed", true));
            }
        }

        #[unsafe(method(centralManager:didDisconnectPeripheral:error:))]
        unsafe fn central_manager_did_disconnect(
            &self,
            _: &CBCentralManager,
            peripheral: &CBPeripheral,
            _error: Option<&NSError>,
        ) {
            if !self.is_active(peripheral) {
                return;
            }
            let disposition =
                self.state_boundary(|boundary, generation| Ok(boundary.disconnected(generation)));
            if let Ok(mut state) = self.ivars().state.lock() {
                state.active = false;
            }
            let expected = matches!(
                disposition,
                Some(Ok(CoreBluetoothDfuApply::Applied(Some(
                    CoreBluetoothDfuEffect::Disconnected(
                        CoreBluetoothDfuDisconnect::AfterActivation
                    )
                ))))
            );
            if expected {
                let _ = self.ivars().disconnected.try_send(Ok(()));
            } else {
                self.fail(failure("ios_dfu_unexpected_disconnect", true));
            }
        }
    }

    unsafe impl CBPeripheralDelegate for DfuDelegate {
        #[unsafe(method(peripheral:didDiscoverServices:))]
        unsafe fn peripheral_did_discover_services(
            &self,
            peripheral: &CBPeripheral,
            error: Option<&NSError>,
        ) {
            if error.is_some() || !self.is_active(peripheral) {
                self.fail(failure("ios_dfu_service_discovery_failed", true));
                return;
            }
            let Some(services) = (unsafe { peripheral.services() }) else {
                self.fail(failure("ios_dfu_service_missing", false));
                return;
            };
            let matching = services
                .to_vec()
                .into_iter()
                .filter(|service| service_uuid(service.as_ref()) == LEGACY_DFU_SERVICE_UUID)
                .collect::<Vec<_>>();
            let [service] = matching.as_slice() else {
                self.fail(failure("ios_dfu_service_missing", false));
                return;
            };
            let control = uuid(LEGACY_DFU_CONTROL_POINT_UUID);
            let packet = uuid(LEGACY_DFU_PACKET_UUID);
            let version = uuid(LEGACY_DFU_VERSION_UUID);
            let characteristics = NSArray::from_slice(&[&*control, &*packet, &*version]);
            unsafe {
                peripheral.discoverCharacteristics_forService(Some(&characteristics), service)
            };
        }

        #[unsafe(method(peripheral:didDiscoverCharacteristicsForService:error:))]
        unsafe fn peripheral_did_discover_characteristics(
            &self,
            peripheral: &CBPeripheral,
            service: &CBService,
            error: Option<&NSError>,
        ) {
            if error.is_some()
                || !self.is_active(peripheral)
                || service_uuid(service) != LEGACY_DFU_SERVICE_UUID
            {
                self.fail(failure("ios_dfu_characteristic_discovery_failed", true));
                return;
            }
            let characteristics = unsafe { service.characteristics() }
                .map(|items| items.to_vec())
                .unwrap_or_default();
            let control = find_characteristic(&characteristics, LEGACY_DFU_CONTROL_POINT_UUID);
            let packet = find_characteristic(&characteristics, LEGACY_DFU_PACKET_UUID);
            let version = find_characteristic(&characteristics, LEGACY_DFU_VERSION_UUID);
            let properties = CoreBluetoothDfuCharacteristics {
                control_point_present: control.is_some(),
                packet_present: packet.is_some(),
                version_present: version.is_some(),
                version_readable: version.is_some_and(|item| {
                    unsafe { item.properties() }.contains(CBCharacteristicProperties::Read)
                }),
                control_write_with_response: control.is_some_and(|item| {
                    unsafe { item.properties() }.contains(CBCharacteristicProperties::Write)
                }),
                notifications_supported: control.is_some_and(|item| {
                    unsafe { item.properties() }.intersects(
                        CBCharacteristicProperties::Notify | CBCharacteristicProperties::Indicate,
                    )
                }),
                packet_write_without_response: packet.is_some_and(|item| {
                    unsafe { item.properties() }
                        .contains(CBCharacteristicProperties::WriteWithoutResponse)
                }),
                maximum_write_value_length: unsafe {
                    peripheral
                        .maximumWriteValueLengthForType(CBCharacteristicWriteType::WithoutResponse)
                        .min(RAK4631_PACKET_BYTES)
                },
            };
            if let Ok(mut state) = self.ivars().state.lock() {
                state.characteristics = Some(properties);
            }
            let Some(control) = control else {
                self.fail(failure("ios_dfu_control_missing", false));
                return;
            };
            unsafe { peripheral.setNotifyValue_forCharacteristic(true, control) };
        }

        #[unsafe(method(peripheral:didUpdateNotificationStateForCharacteristic:error:))]
        unsafe fn peripheral_did_update_notification_state(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            if !self.is_active(peripheral)
                || !characteristic_belongs_to_legacy_service(characteristic)
                || characteristic_uuid(characteristic) != LEGACY_DFU_CONTROL_POINT_UUID
            {
                return;
            }
            if error.is_some() || !unsafe { characteristic.isNotifying() } {
                self.fail(failure("ios_dfu_notifications_failed", true));
                return;
            }
            let Some(version) = find_peripheral_characteristic(peripheral, LEGACY_DFU_VERSION_UUID)
            else {
                self.fail(failure("ios_dfu_version_missing", false));
                return;
            };
            unsafe { peripheral.readValueForCharacteristic(&version) };
        }

        #[unsafe(method(peripheral:didUpdateValueForCharacteristic:error:))]
        unsafe fn peripheral_did_update_value(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            if !self.is_active(peripheral)
                || !characteristic_belongs_to_legacy_service(characteristic)
            {
                return;
            }
            if error.is_some() {
                self.fail(failure("ios_dfu_characteristic_read_failed", true));
                return;
            }
            let Some(data) = (unsafe { characteristic.value() }) else {
                self.fail(failure("ios_dfu_characteristic_value_missing", false));
                return;
            };
            match characteristic_uuid(characteristic).as_str() {
                LEGACY_DFU_VERSION_UUID => {
                    let bytes = data.to_vec();
                    if bytes.len() != 2 {
                        self.fail(failure("ios_dfu_version_invalid", false));
                        return;
                    }
                    let version = u16::from_le_bytes([bytes[0], bytes[1]]);
                    let properties =
                        self.ivars().state.lock().ok().and_then(|state| state.characteristics);
                    let Some(properties) = properties else {
                        self.fail(failure("ios_dfu_characteristics_missing", false));
                        return;
                    };
                    let applied = self.state_boundary(|boundary, generation| {
                        boundary.dfu_discovered(generation, true, properties).map_err(core_failure)
                    });
                    if matches!(applied, Some(Ok(CoreBluetoothDfuApply::Applied(_)))) {
                        let _ = self.ivars().ready.try_send(Ok(version));
                    } else {
                        self.fail(failure("ios_dfu_characteristics_rejected", false));
                    }
                }
                LEGACY_DFU_CONTROL_POINT_UUID => {
                    self.send_notification(data.to_vec());
                }
                _ => {}
            }
        }

        #[unsafe(method(peripheral:didWriteValueForCharacteristic:error:))]
        unsafe fn peripheral_did_write_value(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            if !self.is_active(peripheral)
                || !characteristic_belongs_to_legacy_service(characteristic)
                || characteristic_uuid(characteristic) != LEGACY_DFU_CONTROL_POINT_UUID
            {
                return;
            }
            let result = error
                .map_or_else(|| Ok(()), |_| Err(failure("ios_dfu_control_write_failed", true)));
            if self.ivars().control_writes.try_send(result).is_err() {
                self.fail(failure("ios_dfu_control_callback_queue_full", false));
            }
        }

        #[unsafe(method(peripheralIsReadyToSendWriteWithoutResponse:))]
        unsafe fn peripheral_is_ready_to_send_write_without_response(
            &self,
            peripheral: &CBPeripheral,
        ) {
            if self.is_active(peripheral) {
                let _ = self.ivars().flow_ready.try_send(());
            }
        }
    }
);

impl DfuDelegate {
    fn new(ivars: DfuDelegateIvars) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ivars);
        unsafe { msg_send![super(this), init] }
    }

    fn is_active(&self, peripheral: &CBPeripheral) -> bool {
        let id = peripheral_id_string(peripheral);
        self.ivars()
            .state
            .lock()
            .is_ok_and(|state| state.active && id.as_deref() == Some(&state.peripheral_id))
    }

    fn state_boundary<T>(
        &self,
        apply: impl FnOnce(
            &mut CoreBluetoothDfuBoundary,
            CoreBluetoothGeneration,
        ) -> Result<T, PlatformFailure>,
    ) -> Option<Result<T, PlatformFailure>> {
        self.ivars().state.lock().ok().map(|mut state| {
            let generation = state.generation;
            apply(&mut state.boundary, generation)
        })
    }

    fn manager_was_ready(&self) -> bool {
        self.ivars().state.lock().is_ok_and(|state| state.manager_ready)
    }

    fn fail(&self, error: PlatformFailure) {
        let _ = self.ivars().failures.try_send(error);
    }

    fn send_notification(&self, bytes: Vec<u8>) {
        if self.ivars().notifications.try_send(Ok(bytes)).is_err() {
            self.fail(failure("ios_dfu_notification_queue_full", false));
        }
    }
}

pub struct IosRak4631LegacyDfuGatt {
    delegate: Retained<DfuDelegate>,
    manager: Retained<CBCentralManager>,
    active: RefCell<Option<Retained<CBPeripheral>>>,
    ready: Receiver<Result<u16, PlatformFailure>>,
    notifications: Receiver<Result<Vec<u8>, PlatformFailure>>,
    control_writes: Receiver<Result<(), PlatformFailure>>,
    flow_ready: Receiver<()>,
    disconnected: Receiver<Result<(), PlatformFailure>>,
    failures: Receiver<PlatformFailure>,
    activation_disconnected: bool,
}

impl IosRak4631LegacyDfuGatt {
    async fn connect(
        handoff: IosBleDfuHandoff,
        expected_bytes: usize,
    ) -> Result<(Self, u16), PlatformFailure> {
        let IosBleDfuHandoff { shutdown, peripheral_id } = handoff;
        let boundary =
            CoreBluetoothDfuBoundary::new(shutdown, expected_bytes as u64).map_err(core_failure)?;
        let generation = boundary.generation();
        let (manager_sender, manager_ready) = async_channel::bounded(1);
        let (ready_sender, ready) = async_channel::bounded(1);
        let (notification_sender, notifications) = async_channel::bounded(CHANNEL_CAPACITY);
        let (control_sender, control_writes) = async_channel::bounded(1);
        let (flow_sender, flow_ready) = async_channel::bounded(1);
        let (disconnect_sender, disconnected) = async_channel::bounded(1);
        let (failure_sender, failures) = async_channel::bounded(1);
        let state = DfuState {
            generation,
            peripheral_id: peripheral_id.as_str().to_owned(),
            active: true,
            manager_ready: false,
            boundary,
            characteristics: None,
        };
        let delegate = DfuDelegate::new(DfuDelegateIvars {
            state: Mutex::new(state),
            manager_ready: manager_sender,
            ready: ready_sender,
            notifications: notification_sender,
            control_writes: control_sender,
            flow_ready: flow_sender,
            disconnected: disconnect_sender,
            failures: failure_sender,
        });
        let manager = unsafe {
            CBCentralManager::initWithDelegate_queue(
                CBCentralManager::alloc(),
                Some(ProtocolObject::from_ref(&*delegate)),
                None,
            )
        };
        recv_or_failure(&manager_ready, &failures).await?;
        let peripheral = retrieve_peripheral(&manager, &peripheral_id)?;
        let mut gatt = Self {
            delegate,
            manager,
            active: RefCell::new(Some(peripheral.clone())),
            ready,
            notifications,
            control_writes,
            flow_ready,
            disconnected,
            failures,
            activation_disconnected: false,
        };
        unsafe { gatt.manager.connectPeripheral_options(&peripheral, None) };
        let version = recv_or_failure(&gatt.ready, &gatt.failures).await?;
        gatt.start_write()?;
        Ok((gatt, version))
    }

    fn start_write(&mut self) -> Result<(), PlatformFailure> {
        let mut state = self
            .delegate
            .ivars()
            .state
            .lock()
            .map_err(|_| failure("ios_dfu_state_unavailable", false))?;
        let generation = state.generation;
        state.boundary.write_started(generation).map_err(core_failure)?;
        Ok(())
    }

    fn characteristic(
        &self,
        target: &str,
    ) -> Option<(Retained<CBPeripheral>, Retained<CBCharacteristic>)> {
        let peripheral = self.active.borrow().clone()?;
        let characteristic = find_peripheral_characteristic(&peripheral, target)?;
        Some((peripheral, characteristic))
    }
}

impl LegacyDfuGatt for IosRak4631LegacyDfuGatt {
    fn write_control(&mut self, bytes: Vec<u8>) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(async move {
            let (peripheral, characteristic) =
                self.characteristic(LEGACY_DFU_CONTROL_POINT_UUID)
                    .ok_or_else(|| failure("ios_dfu_control_missing", false))?;
            let data = NSData::from_vec(bytes);
            unsafe {
                peripheral.writeValue_forCharacteristic_type(
                    &data,
                    &characteristic,
                    CBCharacteristicWriteType::WithResponse,
                )
            };
            recv_or_failure(&self.control_writes, &self.failures).await
        })
    }

    fn activate_and_reset(&mut self) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(async move {
            let (peripheral, characteristic) =
                self.characteristic(LEGACY_DFU_CONTROL_POINT_UUID)
                    .ok_or_else(|| failure("ios_dfu_control_missing", false))?;
            let data = NSData::from_vec(vec![0x05]);
            {
                let mut state = self
                    .delegate
                    .ivars()
                    .state
                    .lock()
                    .map_err(|_| failure("ios_dfu_state_unavailable", false))?;
                let generation = state.generation;
                state.boundary.activation_started(generation).map_err(core_failure)?;
                unsafe {
                    peripheral.writeValue_forCharacteristic_type(
                        &data,
                        &characteristic,
                        CBCharacteristicWriteType::WithResponse,
                    )
                };
            }
            match recv_activation(&self.control_writes, &self.disconnected, &self.failures).await? {
                ActivationEvent::WriteCompleted => {}
                ActivationEvent::Disconnected => self.activation_disconnected = true,
            }
            Ok(())
        })
    }

    fn write_packet(&mut self, bytes: Vec<u8>) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(async move {
            if bytes.is_empty() || bytes.len() > RAK4631_PACKET_BYTES {
                return Err(failure("ios_dfu_packet_size_invalid", false));
            }
            let (peripheral, characteristic) = self
                .characteristic(LEGACY_DFU_PACKET_UUID)
                .ok_or_else(|| failure("ios_dfu_packet_missing", false))?;
            while !unsafe { peripheral.canSendWriteWithoutResponse() } {
                recv_flow_or_failure(&self.flow_ready, &self.failures).await?;
            }
            let data = NSData::from_vec(bytes);
            unsafe {
                peripheral.writeValue_forCharacteristic_type(
                    &data,
                    &characteristic,
                    CBCharacteristicWriteType::WithoutResponse,
                )
            };
            Ok(())
        })
    }

    fn notification(&mut self) -> PlatformFuture<'_, Result<Vec<u8>, PlatformFailure>> {
        Box::pin(async move { recv_or_failure(&self.notifications, &self.failures).await })
    }

    fn remote_progress(&mut self, progress: LegacyDfuProgress) -> Result<(), PlatformFailure> {
        let mut state = self
            .delegate
            .ivars()
            .state
            .lock()
            .map_err(|_| failure("ios_dfu_state_unavailable", false))?;
        let generation = state.generation;
        state.boundary.progress_changed(generation, progress.completed).map_err(core_failure)?;
        Ok(())
    }

    fn transfer_completed(&mut self) -> Result<(), PlatformFailure> {
        let mut state = self
            .delegate
            .ivars()
            .state
            .lock()
            .map_err(|_| failure("ios_dfu_state_unavailable", false))?;
        let generation = state.generation;
        state.boundary.write_completed(generation).map_err(core_failure)?;
        Ok(())
    }

    fn wait_disconnected(&mut self) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(async move {
            if self.activation_disconnected {
                return Ok(());
            }
            recv_or_failure(&self.disconnected, &self.failures).await
        })
    }

    fn close(&mut self) {
        if let Some(peripheral) = self.active.replace(None) {
            unsafe { peripheral.setDelegate(None) };
            unsafe { self.manager.cancelPeripheralConnection(&peripheral) };
        }
    }
}

impl Drop for IosRak4631LegacyDfuGatt {
    fn drop(&mut self) {
        LegacyDfuGatt::close(self);
    }
}

/// Execute an admitted RAK4631 Legacy DFU application update on iOS.
///
/// # Errors
///
/// Returns a protocol or CoreBluetooth failure. The caller must apply its own
/// foreground, cancellation, and no-progress deadlines around this future.
pub async fn run_ios_rak4631_legacy_dfu(
    handoff: IosBleDfuHandoff,
    plan: Rak4631LegacyDfuPlan,
    progress: impl FnMut(LegacyDfuProgress),
) -> Result<(), LegacyDfuRunFailure> {
    let expected_bytes = plan.application_len();
    let (mut gatt, version) = IosRak4631LegacyDfuGatt::connect(handoff, expected_bytes)
        .await
        .map_err(LegacyDfuRunFailure::Transport)?;
    run_rak4631_legacy_dfu(plan, version, &mut gatt, progress).await
}

async fn recv_or_failure<T>(
    receiver: &Receiver<Result<T, PlatformFailure>>,
    failures: &Receiver<PlatformFailure>,
) -> Result<T, PlatformFailure> {
    future::race(
        async { receiver.recv().await.map_err(|_| failure("ios_dfu_callback_closed", true))? },
        async {
            Err(failures
                .recv()
                .await
                .unwrap_or_else(|_| failure("ios_dfu_failure_stream_closed", true)))
        },
    )
    .await
}

async fn recv_flow_or_failure(
    receiver: &Receiver<()>,
    failures: &Receiver<PlatformFailure>,
) -> Result<(), PlatformFailure> {
    future::race(
        async { receiver.recv().await.map_err(|_| failure("ios_dfu_flow_closed", true)) },
        async {
            Err(failures
                .recv()
                .await
                .unwrap_or_else(|_| failure("ios_dfu_failure_stream_closed", true)))
        },
    )
    .await
}

enum ActivationEvent {
    WriteCompleted,
    Disconnected,
}

async fn recv_activation(
    writes: &Receiver<Result<(), PlatformFailure>>,
    disconnected: &Receiver<Result<(), PlatformFailure>>,
    failures: &Receiver<PlatformFailure>,
) -> Result<ActivationEvent, PlatformFailure> {
    future::race(
        async { recv_or_failure(writes, failures).await.map(|()| ActivationEvent::WriteCompleted) },
        async {
            recv_or_failure(disconnected, failures).await.map(|()| ActivationEvent::Disconnected)
        },
    )
    .await
}

fn retrieve_peripheral(
    manager: &CBCentralManager,
    id: &BlePeripheralId,
) -> Result<Retained<CBPeripheral>, PlatformFailure> {
    let uuid_string = NSString::from_str(id.as_str());
    let uuid = NSUUID::initWithUUIDString(NSUUID::alloc(), &uuid_string)
        .ok_or_else(|| failure("ios_dfu_peripheral_id_invalid", false))?;
    let identifiers = NSArray::from_slice(&[&*uuid]);
    unsafe { manager.retrievePeripheralsWithIdentifiers(&identifiers) }
        .to_vec()
        .into_iter()
        .next()
        .ok_or_else(|| failure("ios_dfu_peripheral_unavailable", true))
}

fn find_peripheral_characteristic(
    peripheral: &CBPeripheral,
    target: &str,
) -> Option<Retained<CBCharacteristic>> {
    let services = unsafe { peripheral.services() }?
        .to_vec()
        .into_iter()
        .filter(|service| service_uuid(service.as_ref()) == LEGACY_DFU_SERVICE_UUID)
        .collect::<Vec<_>>();
    let [service] = services.as_slice() else {
        return None;
    };
    let characteristics = unsafe { service.characteristics() }?.to_vec();
    let matching = characteristics
        .iter()
        .filter(|item| characteristic_uuid(item.as_ref()) == target)
        .collect::<Vec<_>>();
    let [characteristic] = matching.as_slice() else {
        return None;
    };
    Some((*characteristic).clone())
}

fn find_characteristic<'a>(
    characteristics: &'a [Retained<CBCharacteristic>],
    target: &str,
) -> Option<&'a Retained<CBCharacteristic>> {
    characteristics.iter().find(|item| characteristic_uuid(item.as_ref()) == target)
}

fn uuid(value: &str) -> Retained<CBUUID> {
    let value = NSString::from_str(value);
    unsafe { CBUUID::UUIDWithString(&value) }
}

fn service_uuid(service: &CBService) -> String {
    let uuid = unsafe { service.UUID() };
    unsafe { uuid.UUIDString() }.to_string().to_ascii_uppercase()
}

fn characteristic_uuid(characteristic: &CBCharacteristic) -> String {
    let uuid = unsafe { characteristic.UUID() };
    unsafe { uuid.UUIDString() }.to_string().to_ascii_uppercase()
}

fn characteristic_belongs_to_legacy_service(characteristic: &CBCharacteristic) -> bool {
    unsafe { characteristic.service() }
        .is_some_and(|service| service_uuid(&service) == LEGACY_DFU_SERVICE_UUID)
}

fn peripheral_id_string(peripheral: &CBPeripheral) -> Option<String> {
    let value = unsafe { peripheral.identifier() }.UUIDString().to_string();
    (!value.is_empty()).then_some(value)
}

fn map_manager_state(state: CBManagerState) -> CoreBluetoothManagerState {
    match state {
        CBManagerState::Resetting => CoreBluetoothManagerState::Resetting,
        CBManagerState::Unsupported => CoreBluetoothManagerState::Unsupported,
        CBManagerState::Unauthorized => CoreBluetoothManagerState::Unauthorized,
        CBManagerState::PoweredOff => CoreBluetoothManagerState::PoweredOff,
        CBManagerState::PoweredOn => CoreBluetoothManagerState::PoweredOn,
        _ => CoreBluetoothManagerState::Unknown,
    }
}

fn core_failure(error: CoreBluetoothDfuFailure) -> PlatformFailure {
    PlatformFailure { code: format!("ios_dfu_{error:?}"), retryable: false }
}

fn failure(code: &str, retryable: bool) -> PlatformFailure {
    PlatformFailure { code: code.into(), retryable }
}
