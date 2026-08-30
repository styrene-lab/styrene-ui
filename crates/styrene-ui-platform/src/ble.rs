use crate::{AuthorizationState, PlatformFailure, PlatformFuture};

pub const NORDIC_UART_SERVICE_UUID: &str = "6E400001-B5A3-F393-E0A9-E50E24DCCA9E";
pub const NORDIC_UART_WRITE_UUID: &str = "6E400002-B5A3-F393-E0A9-E50E24DCCA9E";
pub const NORDIC_UART_NOTIFY_UUID: &str = "6E400003-B5A3-F393-E0A9-E50E24DCCA9E";

const MAX_PERIPHERAL_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BleAdapterState {
    Ready,
    PoweredOff,
    Resetting,
    Unsupported,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlePeripheralId(String);

impl BlePeripheralId {
    /// Create a bounded opaque platform peripheral identifier.
    ///
    /// # Errors
    ///
    /// Returns an error when the identifier is empty or exceeds the storage bound.
    pub fn new(value: impl Into<String>) -> Result<Self, BlePeripheralIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(BlePeripheralIdError::Empty);
        }
        if value.len() > MAX_PERIPHERAL_ID_BYTES {
            return Err(BlePeripheralIdError::TooLong);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlePeripheralIdError {
    Empty,
    TooLong,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BleCandidate {
    pub id: BlePeripheralId,
    pub display_name: Option<String>,
    pub rssi_dbm: Option<i16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BleApprovedPeripheral {
    pub id: BlePeripheralId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BleDiscoveryGeneration(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BleDiscoveryEvent {
    Candidate { generation: BleDiscoveryGeneration, candidate: BleCandidate },
    Finished { generation: BleDiscoveryGeneration },
    AdapterChanged { generation: BleDiscoveryGeneration, state: BleAdapterState },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BleDiscoveryApplyResult {
    Applied,
    CapacityReached,
    IgnoredStale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BleScanStartError {
    PermissionRequired,
    PermissionDenied,
    PermissionRestricted,
    PermissionUnavailable,
    AdapterUnavailable(BleAdapterState),
    GenerationExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BleApprovalError {
    StaleGeneration,
    UnknownPeripheral,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BleDiscoveryState {
    generation: u64,
    capacity: usize,
    adapter: BleAdapterState,
    candidates: Vec<BleCandidate>,
    approved: Option<BleApprovedPeripheral>,
}

impl BleDiscoveryState {
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self {
            generation: 0,
            capacity,
            adapter: BleAdapterState::Unavailable,
            candidates: Vec::new(),
            approved: None,
        }
    }

    /// Start a new bounded discovery generation.
    ///
    /// # Errors
    ///
    /// Returns an error when permission or adapter state blocks discovery, or
    /// when the generation counter is exhausted.
    pub fn begin_scan(
        &mut self,
        permission: AuthorizationState,
        adapter: BleAdapterState,
    ) -> Result<BleDiscoveryGeneration, BleScanStartError> {
        match permission {
            AuthorizationState::Granted => {}
            AuthorizationState::NotDetermined => {
                return Err(BleScanStartError::PermissionRequired);
            }
            AuthorizationState::Denied => return Err(BleScanStartError::PermissionDenied),
            AuthorizationState::Restricted => {
                return Err(BleScanStartError::PermissionRestricted);
            }
            AuthorizationState::Unavailable => {
                return Err(BleScanStartError::PermissionUnavailable);
            }
        }
        if adapter != BleAdapterState::Ready {
            return Err(BleScanStartError::AdapterUnavailable(adapter));
        }
        let generation =
            self.generation.checked_add(1).ok_or(BleScanStartError::GenerationExhausted)?;
        self.generation = generation;
        self.adapter = adapter;
        self.candidates.clear();
        Ok(BleDiscoveryGeneration(generation))
    }

    pub fn observe(
        &mut self,
        generation: BleDiscoveryGeneration,
        candidate: BleCandidate,
    ) -> BleDiscoveryApplyResult {
        if generation != BleDiscoveryGeneration(self.generation) {
            return BleDiscoveryApplyResult::IgnoredStale;
        }
        if let Some(existing) = self.candidates.iter_mut().find(|item| item.id == candidate.id) {
            *existing = candidate;
            return BleDiscoveryApplyResult::Applied;
        }
        if self.candidates.len() >= self.capacity {
            return BleDiscoveryApplyResult::CapacityReached;
        }
        self.candidates.push(candidate);
        BleDiscoveryApplyResult::Applied
    }

    pub fn apply_event(&mut self, event: BleDiscoveryEvent) -> BleDiscoveryApplyResult {
        match event {
            BleDiscoveryEvent::Candidate { generation, candidate } => {
                self.observe(generation, candidate)
            }
            BleDiscoveryEvent::Finished { generation } => {
                if generation == BleDiscoveryGeneration(self.generation) {
                    BleDiscoveryApplyResult::Applied
                } else {
                    BleDiscoveryApplyResult::IgnoredStale
                }
            }
            BleDiscoveryEvent::AdapterChanged { generation, state } => {
                if generation != BleDiscoveryGeneration(self.generation) {
                    return BleDiscoveryApplyResult::IgnoredStale;
                }
                self.adapter = state;
                BleDiscoveryApplyResult::Applied
            }
        }
    }

    /// Approve one candidate from the current discovery generation.
    ///
    /// # Errors
    ///
    /// Returns an error for a stale generation or an identifier absent from the
    /// current bounded candidate list.
    pub fn approve(
        &mut self,
        generation: BleDiscoveryGeneration,
        id: &BlePeripheralId,
    ) -> Result<(), BleApprovalError> {
        if generation != BleDiscoveryGeneration(self.generation) {
            return Err(BleApprovalError::StaleGeneration);
        }
        let candidate = self
            .candidates
            .iter()
            .find(|candidate| &candidate.id == id)
            .ok_or(BleApprovalError::UnknownPeripheral)?;
        self.approved = Some(BleApprovedPeripheral { id: candidate.id.clone() });
        Ok(())
    }

    #[must_use]
    pub fn candidates(&self) -> &[BleCandidate] {
        &self.candidates
    }

    #[must_use]
    pub const fn approved(&self) -> Option<&BleApprovedPeripheral> {
        self.approved.as_ref()
    }

    pub fn forget(&mut self) -> bool {
        self.approved.take().is_some()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BleNusProperties {
    pub service_present: bool,
    pub write_with_response: bool,
    pub notifications: bool,
}

impl BleNusProperties {
    /// Validate the required Nordic UART Service properties.
    ///
    /// # Errors
    ///
    /// Returns the first missing service or characteristic property.
    pub fn validate(self) -> Result<(), BleNusError> {
        if !self.service_present {
            return Err(BleNusError::ServiceMissing);
        }
        if !self.write_with_response {
            return Err(BleNusError::WriteWithResponseMissing);
        }
        if !self.notifications {
            return Err(BleNusError::NotificationsMissing);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BleNusError {
    ServiceMissing,
    WriteWithResponseMissing,
    NotificationsMissing,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BleControlPhase {
    #[default]
    Idle,
    Scanning,
    Connecting,
    Connected,
    Reconnecting,
}

impl BleControlPhase {
    #[must_use]
    pub const fn is_busy(self) -> bool {
        matches!(self, Self::Scanning | Self::Connecting | Self::Reconnecting)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BleControlFailure {
    ScanFailed,
    ConnectionInterrupted,
    ConnectionFailed,
    IncompatiblePeripheral,
    PlatformUnavailable,
}

impl BleControlFailure {
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::ConnectionInterrupted | Self::ConnectionFailed)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BleControlDisabledReason {
    PermissionRequired,
    PermissionDenied,
    PermissionRestricted,
    PermissionUnavailable,
    AdapterUnavailable(BleAdapterState),
    OperationInProgress,
    AlreadyConnected,
    NoApprovedPeripheral,
    NoRetryableFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BleControlState {
    pub permission: AuthorizationState,
    pub adapter: BleAdapterState,
    pub phase: BleControlPhase,
    pub candidates: Vec<BleCandidate>,
    pub approved: Option<BleApprovedPeripheral>,
    pub failure: Option<BleControlFailure>,
    pub diagnostic_code: Option<String>,
}

impl Default for BleControlState {
    fn default() -> Self {
        Self {
            permission: AuthorizationState::Unavailable,
            adapter: BleAdapterState::Unavailable,
            phase: BleControlPhase::Idle,
            candidates: Vec::new(),
            approved: None,
            failure: None,
            diagnostic_code: None,
        }
    }
}

impl BleControlState {
    #[must_use]
    pub fn scan_disabled_reason(&self) -> Option<BleControlDisabledReason> {
        match self.permission {
            // The first scan is the user gesture that requests permission. CoreBluetooth does not
            // provide a useful adapter state until that request has started.
            AuthorizationState::NotDetermined => {
                if self.phase == BleControlPhase::Connected {
                    return Some(BleControlDisabledReason::AlreadyConnected);
                }
                return self
                    .phase
                    .is_busy()
                    .then_some(BleControlDisabledReason::OperationInProgress);
            }
            AuthorizationState::Granted => {}
            AuthorizationState::Denied => return Some(BleControlDisabledReason::PermissionDenied),
            AuthorizationState::Restricted => {
                return Some(BleControlDisabledReason::PermissionRestricted);
            }
            AuthorizationState::Unavailable => {
                return Some(BleControlDisabledReason::PermissionUnavailable);
            }
        }
        if self.adapter != BleAdapterState::Ready {
            return Some(BleControlDisabledReason::AdapterUnavailable(self.adapter));
        }
        if self.phase == BleControlPhase::Connected {
            return Some(BleControlDisabledReason::AlreadyConnected);
        }
        if self.phase.is_busy() {
            return Some(BleControlDisabledReason::OperationInProgress);
        }
        None
    }

    #[must_use]
    pub fn selection_disabled_reason(&self) -> Option<BleControlDisabledReason> {
        if self.permission == AuthorizationState::NotDetermined {
            return Some(BleControlDisabledReason::PermissionRequired);
        }
        if self.phase.is_busy() {
            Some(BleControlDisabledReason::OperationInProgress)
        } else {
            self.scan_disabled_reason()
        }
    }

    #[must_use]
    pub fn retry_disabled_reason(&self) -> Option<BleControlDisabledReason> {
        if self.phase.is_busy() {
            return Some(BleControlDisabledReason::OperationInProgress);
        }
        if self.approved.is_none() {
            return Some(BleControlDisabledReason::NoApprovedPeripheral);
        }
        if !self.failure.as_ref().is_some_and(BleControlFailure::is_retryable) {
            return Some(BleControlDisabledReason::NoRetryableFailure);
        }
        self.scan_disabled_reason()
    }

    #[must_use]
    pub fn forget_disabled_reason(&self) -> Option<BleControlDisabledReason> {
        if self.approved.is_none() {
            Some(BleControlDisabledReason::NoApprovedPeripheral)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BleWriteLimit(usize);

impl BleWriteLimit {
    /// Create a nonzero platform write limit.
    ///
    /// # Errors
    ///
    /// Returns an error when `bytes` is zero.
    pub fn new(bytes: usize) -> Result<Self, BleWriteLimitError> {
        if bytes == 0 {
            return Err(BleWriteLimitError::Zero);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn bytes(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BleWriteLimitError {
    Zero,
}

/// One connected NUS attempt. A write completes only after its response arrives.
pub trait BleRNodeByteAttempt {
    fn read(&self) -> PlatformFuture<'_, Result<Option<Vec<u8>>, PlatformFailure>>;
    fn write_with_response(&self, data: Vec<u8>)
    -> PlatformFuture<'_, Result<(), PlatformFailure>>;
    fn close(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthorizationState;

    fn candidate(id: &str, name: &str) -> BleCandidate {
        BleCandidate {
            id: BlePeripheralId::new(id).unwrap(),
            display_name: Some(name.into()),
            rssi_dbm: Some(-55),
        }
    }

    #[test]
    fn scan_requires_permission_and_available_adapter_and_bounds_candidates() {
        let mut state = BleDiscoveryState::new(2);
        assert_eq!(
            state.begin_scan(AuthorizationState::Denied, BleAdapterState::Ready),
            Err(BleScanStartError::PermissionDenied)
        );
        assert_eq!(
            state.begin_scan(AuthorizationState::Granted, BleAdapterState::PoweredOff),
            Err(BleScanStartError::AdapterUnavailable(BleAdapterState::PoweredOff))
        );

        let generation =
            state.begin_scan(AuthorizationState::Granted, BleAdapterState::Ready).unwrap();
        assert_eq!(
            state.observe(generation, candidate("peripheral-a", "RNode A")),
            BleDiscoveryApplyResult::Applied
        );
        assert_eq!(
            state.observe(generation, candidate("peripheral-b", "RNode B")),
            BleDiscoveryApplyResult::Applied
        );
        assert_eq!(
            state.observe(generation, candidate("peripheral-c", "RNode C")),
            BleDiscoveryApplyResult::CapacityReached
        );
        assert_eq!(state.candidates().len(), 2);

        let updated = candidate("peripheral-a", "Updated A");
        assert_eq!(state.observe(generation, updated.clone()), BleDiscoveryApplyResult::Applied);
        assert_eq!(state.candidates()[0], updated);
    }

    #[test]
    fn approval_is_explicit_forgettable_and_generation_safe() {
        let mut state = BleDiscoveryState::new(4);
        let first = state.begin_scan(AuthorizationState::Granted, BleAdapterState::Ready).unwrap();
        state.observe(first, candidate("peripheral-a", "Shared Name"));
        state.observe(first, candidate("peripheral-b", "Shared Name"));
        assert_eq!(state.approved(), None);
        assert_eq!(
            state.approve(first, &BlePeripheralId::new("missing").unwrap()),
            Err(BleApprovalError::UnknownPeripheral)
        );
        assert_eq!(state.approve(first, &BlePeripheralId::new("peripheral-b").unwrap()), Ok(()));
        assert_eq!(state.approved().unwrap().id.as_str(), "peripheral-b");

        let second = state.begin_scan(AuthorizationState::Granted, BleAdapterState::Ready).unwrap();
        assert_eq!(
            state.observe(first, candidate("stale", "Stale")),
            BleDiscoveryApplyResult::IgnoredStale
        );
        assert_eq!(
            state.approve(first, &BlePeripheralId::new("peripheral-a").unwrap()),
            Err(BleApprovalError::StaleGeneration)
        );
        assert!(state.candidates().is_empty());
        assert_eq!(state.approved().unwrap().id.as_str(), "peripheral-b");

        state.observe(second, candidate("peripheral-b", "Renamed"));
        assert_eq!(state.approved().unwrap().id.as_str(), "peripheral-b");
        assert!(state.forget());
        assert_eq!(state.approved(), None);
        assert!(!state.forget());
    }

    #[test]
    fn nus_properties_and_write_limit_fail_closed() {
        assert_eq!(
            BleNusProperties {
                service_present: true,
                write_with_response: false,
                notifications: true,
            }
            .validate(),
            Err(BleNusError::WriteWithResponseMissing)
        );
        assert_eq!(BleWriteLimit::new(0), Err(BleWriteLimitError::Zero));
        assert_eq!(BleWriteLimit::new(185).unwrap().bytes(), 185);
        assert_eq!(NORDIC_UART_SERVICE_UUID.len(), 36);
        assert_eq!(NORDIC_UART_WRITE_UUID.len(), 36);
        assert_eq!(NORDIC_UART_NOTIFY_UUID.len(), 36);
    }

    #[test]
    fn control_disabled_reasons_are_typed_and_forget_remains_reachable() {
        let mut state = BleControlState::default();
        assert_eq!(
            state.scan_disabled_reason(),
            Some(BleControlDisabledReason::PermissionUnavailable)
        );

        state.permission = AuthorizationState::NotDetermined;
        assert_eq!(state.scan_disabled_reason(), None);
        state.adapter = BleAdapterState::Ready;
        assert_eq!(state.scan_disabled_reason(), None);
        assert_eq!(
            state.selection_disabled_reason(),
            Some(BleControlDisabledReason::PermissionRequired)
        );

        state.permission = AuthorizationState::Granted;
        state.failure = Some(BleControlFailure::ScanFailed);
        assert!(!state.failure.as_ref().unwrap().is_retryable());
        state.failure = Some(BleControlFailure::ConnectionInterrupted);
        assert_eq!(
            state.retry_disabled_reason(),
            Some(BleControlDisabledReason::NoApprovedPeripheral)
        );
        state.approved =
            Some(BleApprovedPeripheral { id: BlePeripheralId::new("approved-rnode").unwrap() });
        assert_eq!(state.retry_disabled_reason(), None);
        state.phase = BleControlPhase::Reconnecting;
        assert_eq!(
            state.retry_disabled_reason(),
            Some(BleControlDisabledReason::OperationInProgress)
        );
        assert_eq!(state.forget_disabled_reason(), None);

        state.phase = BleControlPhase::Connected;
        assert_eq!(state.scan_disabled_reason(), Some(BleControlDisabledReason::AlreadyConnected));
        assert_eq!(
            state.selection_disabled_reason(),
            Some(BleControlDisabledReason::AlreadyConnected)
        );
    }
}
