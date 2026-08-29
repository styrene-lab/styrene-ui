use std::cell::RefCell;
use std::sync::Mutex;

use async_channel::{Receiver, Sender};
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
    NSUserDefaults,
};
use styrene_ui_platform::{
    BleAdapterState, BleCandidate, BlePeripheralId, BleRNodeByteAttempt, BleWriteLimit,
    NORDIC_UART_NOTIFY_UUID, NORDIC_UART_SERVICE_UUID, NORDIC_UART_WRITE_UUID, PlatformFailure,
    PlatformFuture,
};

use crate::{
    CoreBluetoothApply, CoreBluetoothAttemptBoundary, CoreBluetoothEffect, CoreBluetoothFailure,
    CoreBluetoothGeneration, CoreBluetoothManagerState, CoreBluetoothNusCharacteristics,
    CoreBluetoothWriteToken,
};

const EVENT_CAPACITY: usize = 64;
const BYTE_CAPACITY: usize = 64;
const APPROVED_PERIPHERAL_KEY: &str = "io.styrene.mesh.approved-ble-peripheral";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IosBleEvent {
    AdapterChanged(BleAdapterState),
    Candidate { generation: CoreBluetoothGeneration, candidate: BleCandidate },
    Ready { generation: CoreBluetoothGeneration, write_limit: BleWriteLimit },
    Failed { generation: CoreBluetoothGeneration, failure: CoreBluetoothFailure },
    Disconnected { generation: CoreBluetoothGeneration },
}

pub struct IosBleEventStream {
    receiver: Receiver<IosBleEvent>,
}

impl IosBleEventStream {
    pub async fn next(&self) -> Option<IosBleEvent> {
        self.receiver.recv().await.ok()
    }
}

struct DelegateState {
    scan_generation: Option<CoreBluetoothGeneration>,
    scanning: bool,
    active_generation: Option<CoreBluetoothGeneration>,
    active_id: Option<String>,
    boundary: Option<CoreBluetoothAttemptBoundary>,
    pending_write: Option<CoreBluetoothWriteToken>,
}

struct BleDelegateIvars {
    events: Sender<IosBleEvent>,
    reads: Sender<Result<Vec<u8>, PlatformFailure>>,
    writes: Sender<Result<(), PlatformFailure>>,
    state: Mutex<DelegateState>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements. All ivars are initialized
    // before `init` returns and contain no Objective-C references.
    #[unsafe(super = NSObject)]
    #[ivars = BleDelegateIvars]
    struct BleDelegate;

    // SAFETY: NSObjectProtocol adds no implementation requirements.
    unsafe impl NSObjectProtocol for BleDelegate {}

    // SAFETY: Every selector and argument matches CBCentralManagerDelegate.
    unsafe impl CBCentralManagerDelegate for BleDelegate {
        #[unsafe(method(centralManagerDidUpdateState:))]
        unsafe fn central_manager_did_update_state(&self, central: &CBCentralManager) {
            let state = map_manager_state(unsafe { central.state() });
            self.send_event(IosBleEvent::AdapterChanged(state.adapter_state()));
        }

        #[unsafe(method(centralManager:didDiscoverPeripheral:advertisementData:RSSI:))]
        unsafe fn central_manager_did_discover(
            &self,
            _: &CBCentralManager,
            peripheral: &CBPeripheral,
            _: &NSDictionary<NSString, objc2::runtime::AnyObject>,
            rssi: &NSNumber,
        ) {
            let Some(id) = peripheral_id(peripheral) else { return };
            let generation = self
                .ivars()
                .state
                .lock()
                .ok()
                .and_then(|state| state.scanning.then_some(state.scan_generation).flatten());
            let Some(generation) = generation else { return };
            let candidate = BleCandidate {
                id,
                display_name: unsafe { peripheral.name() }.map(|name| name.to_string()),
                rssi_dbm: (rssi.shortValue() != 127).then(|| rssi.shortValue()),
            };
            self.send_event(IosBleEvent::Candidate { generation, candidate });
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
            unsafe {
                peripheral.setDelegate(Some(ProtocolObject::from_ref(self)));
            }
            let service = nus_uuid(NORDIC_UART_SERVICE_UUID);
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
            self.finish_connection(peripheral, CoreBluetoothFailure::ConnectionFailed);
        }

        #[unsafe(method(centralManager:didDisconnectPeripheral:error:))]
        unsafe fn central_manager_did_disconnect(
            &self,
            _: &CBCentralManager,
            peripheral: &CBPeripheral,
            _: Option<&NSError>,
        ) {
            let generation = self.take_active(peripheral);
            if let Some(generation) = generation {
                self.send_event(IosBleEvent::Disconnected { generation });
                let _ = self.reads().try_send(Err(failure("ios_ble_disconnected", true)));
            }
        }
    }

    // SAFETY: Every selector and argument matches CBPeripheralDelegate.
    unsafe impl CBPeripheralDelegate for BleDelegate {
        #[unsafe(method(peripheral:didDiscoverServices:))]
        unsafe fn peripheral_did_discover_services(
            &self,
            peripheral: &CBPeripheral,
            error: Option<&NSError>,
        ) {
            if error.is_some() || !self.is_active(peripheral) {
                self.fail_active(CoreBluetoothFailure::NusServiceMissing);
                return;
            }
            let Some(services) = (unsafe { peripheral.services() }) else {
                self.fail_active(CoreBluetoothFailure::NusServiceMissing);
                return;
            };
            let service = services
                .to_vec()
                .into_iter()
                .find(|service| service_uuid(service.as_ref()) == NORDIC_UART_SERVICE_UUID);
            let Some(service) = service else {
                self.apply_services(false);
                return;
            };
            if self.apply_services(true) {
                let write = nus_uuid(NORDIC_UART_WRITE_UUID);
                let notify = nus_uuid(NORDIC_UART_NOTIFY_UUID);
                let characteristics = NSArray::from_slice(&[&*write, &*notify]);
                unsafe {
                    peripheral.discoverCharacteristics_forService(Some(&characteristics), &service);
                }
            }
        }

        #[unsafe(method(peripheral:didDiscoverCharacteristicsForService:error:))]
        unsafe fn peripheral_did_discover_characteristics(
            &self,
            peripheral: &CBPeripheral,
            service: &CBService,
            error: Option<&NSError>,
        ) {
            if error.is_some() || !self.is_active(peripheral) {
                self.fail_active(CoreBluetoothFailure::NotifyCharacteristicMissing);
                return;
            }
            let characteristics = unsafe { service.characteristics() }
                .map(|items| items.to_vec())
                .unwrap_or_default();
            let write = characteristics
                .iter()
                .find(|item| characteristic_uuid(item.as_ref()) == NORDIC_UART_WRITE_UUID);
            let notify = characteristics
                .iter()
                .find(|item| characteristic_uuid(item.as_ref()) == NORDIC_UART_NOTIFY_UUID);
            let properties = CoreBluetoothNusCharacteristics {
                write_characteristic_present: write.is_some(),
                notify_characteristic_present: notify.is_some(),
                write_with_response: write.is_some_and(|item| {
                    unsafe { item.properties() }.contains(CBCharacteristicProperties::Write)
                }),
                notifications_supported: notify.is_some_and(|item| {
                    unsafe { item.properties() }.intersects(
                        CBCharacteristicProperties::Notify | CBCharacteristicProperties::Indicate,
                    )
                }),
                maximum_write_value_length: unsafe {
                    peripheral
                        .maximumWriteValueLengthForType(CBCharacteristicWriteType::WithResponse)
                },
            };
            if self.apply_characteristics(properties)
                && let Some(notify) = notify
            {
                unsafe { peripheral.setNotifyValue_forCharacteristic(true, notify) };
            }
        }

        #[unsafe(method(peripheral:didUpdateNotificationStateForCharacteristic:error:))]
        unsafe fn peripheral_did_update_notification_state(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            if !self.is_active(peripheral)
                || characteristic_uuid(characteristic) != NORDIC_UART_NOTIFY_UUID
            {
                return;
            }
            self.apply_notifications(error.is_none() && unsafe { characteristic.isNotifying() });
        }

        #[unsafe(method(peripheral:didUpdateValueForCharacteristic:error:))]
        unsafe fn peripheral_did_update_value(
            &self,
            peripheral: &CBPeripheral,
            characteristic: &CBCharacteristic,
            error: Option<&NSError>,
        ) {
            if !self.is_active(peripheral)
                || characteristic_uuid(characteristic) != NORDIC_UART_NOTIFY_UUID
            {
                return;
            }
            if error.is_some() {
                let _ = self.reads().try_send(Err(failure("ios_ble_notification_failed", true)));
                return;
            }
            let Some(data) = (unsafe { characteristic.value() }) else { return };
            let bytes = data.to_vec();
            if self.apply_notification(bytes.clone()) {
                let _ = self.reads().try_send(Ok(bytes)).or_else(|_| {
                    self.reads()
                        .force_send(Err(failure("ios_ble_read_queue_full", false)))
                        .map(|_| ())
                });
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
                || characteristic_uuid(characteristic) != NORDIC_UART_WRITE_UUID
            {
                return;
            }
            let result = self.apply_write_completion(error.is_none());
            let _ = self.writes().try_send(result);
        }
    }
);

impl BleDelegate {
    fn new(
        events: Sender<IosBleEvent>,
        reads: Sender<Result<Vec<u8>, PlatformFailure>>,
        writes: Sender<Result<(), PlatformFailure>>,
    ) -> Retained<Self> {
        let this = Self::alloc().set_ivars(BleDelegateIvars {
            events,
            reads,
            writes,
            state: Mutex::new(DelegateState {
                scan_generation: None,
                scanning: false,
                active_generation: None,
                active_id: None,
                boundary: None,
                pending_write: None,
            }),
        });
        // SAFETY: NSObject's parameterless init receives fully initialized ivars.
        unsafe { msg_send![super(this), init] }
    }

    fn reads(&self) -> &Sender<Result<Vec<u8>, PlatformFailure>> {
        &self.ivars().reads
    }

    fn writes(&self) -> &Sender<Result<(), PlatformFailure>> {
        &self.ivars().writes
    }

    fn send_event(&self, event: IosBleEvent) {
        let _ = self.ivars().events.force_send(event);
    }

    fn is_active(&self, peripheral: &CBPeripheral) -> bool {
        let id = peripheral_id_string(peripheral);
        self.ivars()
            .state
            .lock()
            .is_ok_and(|state| state.active_generation.is_some() && state.active_id == id)
    }

    fn apply_services(&self, present: bool) -> bool {
        self.apply_boundary(|boundary, generation| {
            boundary.services_discovered(generation, present)
        })
    }

    fn apply_characteristics(&self, properties: CoreBluetoothNusCharacteristics) -> bool {
        self.apply_boundary(|boundary, generation| {
            boundary.characteristics_discovered(generation, properties)
        })
    }

    fn apply_notifications(&self, enabled: bool) -> bool {
        self.apply_boundary(|boundary, generation| {
            boundary.notifications_changed(generation, enabled)
        })
    }

    fn apply_notification(&self, bytes: Vec<u8>) -> bool {
        self.apply_boundary(|boundary, generation| boundary.notification(generation, bytes))
    }

    fn apply_boundary(
        &self,
        apply: impl FnOnce(
            &mut CoreBluetoothAttemptBoundary,
            CoreBluetoothGeneration,
        ) -> Result<CoreBluetoothApply, CoreBluetoothFailure>,
    ) -> bool {
        let outcome = self.ivars().state.lock().ok().and_then(|mut state| {
            let generation = state.active_generation?;
            let boundary = state.boundary.as_mut()?;
            Some((generation, apply(boundary, generation)))
        });
        match outcome {
            Some((generation, Ok(CoreBluetoothApply::Applied(effects)))) => {
                for effect in effects {
                    if let CoreBluetoothEffect::Ready(write_limit) = effect {
                        self.send_event(IosBleEvent::Ready { generation, write_limit });
                    }
                }
                true
            }
            Some((generation, Err(failure))) => {
                self.send_event(IosBleEvent::Failed { generation, failure });
                false
            }
            _ => false,
        }
    }

    fn fail_active(&self, failure: CoreBluetoothFailure) {
        let generation = self.ivars().state.lock().ok().and_then(|state| state.active_generation);
        if let Some(generation) = generation {
            self.send_event(IosBleEvent::Failed { generation, failure });
        }
    }

    fn finish_connection(&self, peripheral: &CBPeripheral, failure: CoreBluetoothFailure) {
        if let Some(generation) = self.take_active(peripheral) {
            self.send_event(IosBleEvent::Failed { generation, failure });
        }
    }

    fn take_active(&self, peripheral: &CBPeripheral) -> Option<CoreBluetoothGeneration> {
        let id = peripheral_id_string(peripheral);
        let mut state = self.ivars().state.lock().ok()?;
        if state.active_id != id {
            return None;
        }
        let generation = state.active_generation.take()?;
        state.active_id = None;
        state.boundary = None;
        state.pending_write = None;
        Some(generation)
    }

    fn apply_write_completion(&self, succeeded: bool) -> Result<(), PlatformFailure> {
        let result = self.ivars().state.lock().ok().and_then(|mut state| {
            let generation = state.active_generation?;
            let token = state.pending_write.take()?;
            let boundary = state.boundary.as_mut()?;
            Some(boundary.write_completed(generation, token, succeeded))
        });
        match result {
            Some(Ok(CoreBluetoothApply::Applied(_))) => Ok(()),
            Some(Err(CoreBluetoothFailure::WriteFailed)) => {
                Err(failure("ios_ble_write_failed", true))
            }
            _ => Err(failure("ios_ble_write_callback_mismatch", false)),
        }
    }
}

pub struct IosBleAdapter {
    delegate: Retained<BleDelegate>,
    manager: Retained<CBCentralManager>,
    active: RefCell<Option<Retained<CBPeripheral>>>,
    events: Option<Receiver<IosBleEvent>>,
    reads: Receiver<Result<Vec<u8>, PlatformFailure>>,
    writes: Receiver<Result<(), PlatformFailure>>,
}

impl IosBleAdapter {
    #[must_use]
    pub fn new() -> Self {
        let (event_sender, events) = async_channel::bounded(EVENT_CAPACITY);
        let (read_sender, reads) = async_channel::bounded(BYTE_CAPACITY);
        let (write_sender, writes) = async_channel::bounded(1);
        let delegate = BleDelegate::new(event_sender, read_sender, write_sender);
        // SAFETY: The adapter retains the manager and delegate for the same lifetime.
        // A nil queue selects Apple's serial main dispatch queue.
        let manager = unsafe {
            CBCentralManager::initWithDelegate_queue(
                CBCentralManager::alloc(),
                Some(ProtocolObject::from_ref(&*delegate)),
                None,
            )
        };
        Self { delegate, manager, active: RefCell::new(None), events: Some(events), reads, writes }
    }

    pub fn take_event_stream(&mut self) -> Option<IosBleEventStream> {
        self.events.take().map(|receiver| IosBleEventStream { receiver })
    }

    pub fn start_scan(&self, generation: CoreBluetoothGeneration) -> Result<(), PlatformFailure> {
        if map_manager_state(unsafe { self.manager.state() })
            != CoreBluetoothManagerState::PoweredOn
        {
            return Err(failure("ios_ble_adapter_unavailable", true));
        }
        {
            let mut state = self
                .delegate
                .ivars()
                .state
                .lock()
                .map_err(|_| failure("ios_ble_state_unavailable", false))?;
            if state.scanning || state.scan_generation.is_some_and(|current| current != generation)
            {
                return Err(failure("ios_ble_scan_generation_closed", true));
            }
            state.scan_generation = Some(generation);
            state.scanning = true;
        }
        let service = nus_uuid(NORDIC_UART_SERVICE_UUID);
        let services = NSArray::from_slice(&[&*service]);
        // SAFETY: The typed array contains only the canonical NUS CBUUID.
        unsafe { self.manager.scanForPeripheralsWithServices_options(Some(&services), None) };
        Ok(())
    }

    pub fn stop_scan(&self) {
        if let Ok(mut state) = self.delegate.ivars().state.lock() {
            state.scanning = false;
        }
        // SAFETY: Stopping an inactive scan is documented as harmless.
        unsafe { self.manager.stopScan() };
    }

    pub fn connect(
        &self,
        generation: CoreBluetoothGeneration,
        id: &BlePeripheralId,
    ) -> Result<(), PlatformFailure> {
        if map_manager_state(unsafe { self.manager.state() })
            != CoreBluetoothManagerState::PoweredOn
        {
            return Err(failure("ios_ble_adapter_unavailable", true));
        }
        while self.reads.try_recv().is_ok() {}
        while self.writes.try_recv().is_ok() {}
        let uuid_string = NSString::from_str(id.as_str());
        let uuid = NSUUID::initWithUUIDString(NSUUID::alloc(), &uuid_string)
            .ok_or_else(|| failure("ios_ble_peripheral_id_invalid", false))?;
        let identifiers = NSArray::from_slice(&[&*uuid]);
        let peripherals = unsafe { self.manager.retrievePeripheralsWithIdentifiers(&identifiers) };
        let peripheral = peripherals
            .to_vec()
            .into_iter()
            .next()
            .ok_or_else(|| failure("ios_ble_peripheral_unavailable", true))?;
        {
            let mut state = self
                .delegate
                .ivars()
                .state
                .lock()
                .map_err(|_| failure("ios_ble_state_unavailable", false))?;
            if state.active_generation.is_some() {
                return Err(failure("ios_ble_attempt_active", true));
            }
            let mut boundary = CoreBluetoothAttemptBoundary::new(generation);
            boundary
                .manager_changed(generation, CoreBluetoothManagerState::PoweredOn)
                .map_err(core_failure)?;
            state.active_generation = Some(generation);
            state.active_id = Some(id.as_str().to_owned());
            state.boundary = Some(boundary);
            state.pending_write = None;
        }
        self.stop_scan();
        self.active.replace(Some(peripheral.clone()));
        // SAFETY: The adapter retains the retrieved peripheral until terminal close.
        unsafe { self.manager.connectPeripheral_options(&peripheral, None) };
        Ok(())
    }

    fn write_characteristic(&self) -> Option<(Retained<CBPeripheral>, Retained<CBCharacteristic>)> {
        let peripheral = self.active.borrow().clone()?;
        let services = unsafe { peripheral.services() }?;
        for service in services.to_vec() {
            let characteristics = unsafe { service.characteristics() }?;
            if let Some(characteristic) = characteristics
                .to_vec()
                .into_iter()
                .find(|item| characteristic_uuid(item.as_ref()) == NORDIC_UART_WRITE_UUID)
            {
                return Some((peripheral, characteristic));
            }
        }
        None
    }
}

#[must_use]
pub fn approved_ble_peripheral() -> Option<BlePeripheralId> {
    let key = NSString::from_str(APPROVED_PERIPHERAL_KEY);
    let value = NSUserDefaults::standardUserDefaults().stringForKey(&key)?;
    BlePeripheralId::new(value.to_string()).ok()
}

pub fn store_approved_ble_peripheral(id: Option<&BlePeripheralId>) {
    let defaults = NSUserDefaults::standardUserDefaults();
    let key = NSString::from_str(APPROVED_PERIPHERAL_KEY);
    if let Some(id) = id {
        let value = NSString::from_str(id.as_str());
        // SAFETY: NSString is a supported NSUserDefaults property-list value.
        unsafe { defaults.setObject_forKey(Some(&value), &key) };
    } else {
        defaults.removeObjectForKey(&key);
    }
}

impl Default for IosBleAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl BleRNodeByteAttempt for IosBleAdapter {
    fn read(&self) -> PlatformFuture<'_, Result<Option<Vec<u8>>, PlatformFailure>> {
        Box::pin(async move {
            match self.reads.recv().await {
                Ok(Ok(bytes)) => Ok(Some(bytes)),
                Ok(Err(error)) => Err(error),
                Err(_) => Err(failure("ios_ble_read_closed", true)),
            }
        })
    }

    fn write_with_response(
        &self,
        bytes: Vec<u8>,
    ) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
        Box::pin(async move {
            if map_manager_state(unsafe { self.manager.state() })
                != CoreBluetoothManagerState::PoweredOn
            {
                return Err(failure("ios_ble_adapter_unavailable", true));
            }
            let request = {
                let mut state = self
                    .delegate
                    .ivars()
                    .state
                    .lock()
                    .map_err(|_| failure("ios_ble_state_unavailable", false))?;
                let boundary = state
                    .boundary
                    .as_mut()
                    .ok_or_else(|| failure("ios_ble_attempt_inactive", true))?;
                let request = boundary.begin_write(bytes).map_err(core_failure)?;
                state.pending_write = Some(request.token);
                request
            };
            let (peripheral, characteristic) = self
                .write_characteristic()
                .ok_or_else(|| failure("ios_ble_write_characteristic_unavailable", false))?;
            let data = NSData::from_vec(request.bytes);
            unsafe {
                peripheral.writeValue_forCharacteristic_type(
                    &data,
                    &characteristic,
                    CBCharacteristicWriteType::WithResponse,
                );
            }
            self.writes.recv().await.map_err(|_| failure("ios_ble_write_callback_closed", true))?
        })
    }

    fn close(&mut self) {
        self.stop_scan();
        if let Some(peripheral) = self.active.borrow().as_ref() {
            unsafe { peripheral.setDelegate(None) };
            unsafe { self.manager.cancelPeripheralConnection(peripheral) };
        }
        if let Ok(mut state) = self.delegate.ivars().state.lock()
            && let Some(boundary) = state.boundary.as_mut()
        {
            boundary.close();
            state.pending_write = None;
        }
    }
}

impl Drop for IosBleAdapter {
    fn drop(&mut self) {
        BleRNodeByteAttempt::close(self);
    }
}

fn nus_uuid(value: &str) -> Retained<CBUUID> {
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

fn peripheral_id(peripheral: &CBPeripheral) -> Option<BlePeripheralId> {
    BlePeripheralId::new(peripheral_id_string(peripheral)?).ok()
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

fn core_failure(failure: CoreBluetoothFailure) -> PlatformFailure {
    let retryable = matches!(
        failure,
        CoreBluetoothFailure::ManagerUnavailable(_)
            | CoreBluetoothFailure::ConnectionFailed
            | CoreBluetoothFailure::WriteFailed
            | CoreBluetoothFailure::NotificationSubscriptionFailed
    );
    PlatformFailure { code: format!("ios_ble_{failure:?}"), retryable }
}

fn failure(code: &str, retryable: bool) -> PlatformFailure {
    PlatformFailure { code: code.into(), retryable }
}
