use styrene_ui_platform::{BleAdapterState, BleWriteLimit};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreBluetoothGeneration(u64);

impl CoreBluetoothGeneration {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CoreBluetoothNusShutdown {
    generation: CoreBluetoothGeneration,
}

impl CoreBluetoothNusShutdown {
    pub(crate) const fn generation(self) -> CoreBluetoothGeneration {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothManagerState {
    Unknown,
    Resetting,
    Unsupported,
    Unauthorized,
    PoweredOff,
    PoweredOn,
}

impl CoreBluetoothManagerState {
    #[must_use]
    pub const fn adapter_state(self) -> BleAdapterState {
        match self {
            Self::PoweredOn => BleAdapterState::Ready,
            Self::PoweredOff => BleAdapterState::PoweredOff,
            Self::Resetting => BleAdapterState::Resetting,
            Self::Unsupported => BleAdapterState::Unsupported,
            Self::Unknown | Self::Unauthorized => BleAdapterState::Unavailable,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreBluetoothNusCharacteristics {
    pub write_characteristic_present: bool,
    pub notify_characteristic_present: bool,
    pub write_with_response: bool,
    pub notifications_supported: bool,
    pub maximum_write_value_length: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreBluetoothWriteToken(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreBluetoothWriteRequest {
    pub token: CoreBluetoothWriteToken,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothPhase {
    WaitingForManager,
    DiscoveringServices,
    DiscoveringCharacteristics,
    EnablingNotifications,
    Ready,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothFailure {
    ManagerUnavailable(CoreBluetoothManagerState),
    NusServiceMissing,
    WriteCharacteristicMissing,
    NotifyCharacteristicMissing,
    WriteWithResponseMissing,
    NotificationsUnsupported,
    ConnectionFailed,
    InvalidWriteLimit,
    InvalidPhase,
    WriteInProgress,
    WriteTooLarge { maximum: usize, actual: usize },
    WriteCallbackMismatch,
    WriteFailed,
    NotificationSubscriptionFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreBluetoothEffect {
    AdapterChanged(BleAdapterState),
    DiscoverNusService,
    DiscoverNusCharacteristics,
    EnableNotifications,
    Ready(BleWriteLimit),
    Notification(Vec<u8>),
    WriteCompleted(CoreBluetoothWriteToken),
    Disconnected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreBluetoothApply {
    Applied(Vec<CoreBluetoothEffect>),
    IgnoredStale,
}

#[derive(Debug)]
pub struct CoreBluetoothAttemptBoundary {
    generation: CoreBluetoothGeneration,
    phase: CoreBluetoothPhase,
    write_limit: Option<BleWriteLimit>,
    next_write: u64,
    pending_write: Option<CoreBluetoothWriteToken>,
    disconnection_observed: bool,
    shutdown_transferred: bool,
}

impl CoreBluetoothAttemptBoundary {
    #[must_use]
    pub const fn new(generation: CoreBluetoothGeneration) -> Self {
        Self {
            generation,
            phase: CoreBluetoothPhase::WaitingForManager,
            write_limit: None,
            next_write: 1,
            pending_write: None,
            disconnection_observed: false,
            shutdown_transferred: false,
        }
    }

    #[must_use]
    pub const fn phase(&self) -> CoreBluetoothPhase {
        self.phase
    }

    pub fn manager_changed(
        &mut self,
        generation: CoreBluetoothGeneration,
        state: CoreBluetoothManagerState,
    ) -> Result<CoreBluetoothApply, CoreBluetoothFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothApply::IgnoredStale);
        }
        if self.shutdown_transferred {
            return Err(CoreBluetoothFailure::InvalidPhase);
        }
        if state != CoreBluetoothManagerState::PoweredOn {
            self.phase = CoreBluetoothPhase::WaitingForManager;
            self.write_limit = None;
            self.pending_write = None;
            return Err(CoreBluetoothFailure::ManagerUnavailable(state));
        }
        self.phase = CoreBluetoothPhase::DiscoveringServices;
        Ok(CoreBluetoothApply::Applied(vec![
            CoreBluetoothEffect::AdapterChanged(state.adapter_state()),
            CoreBluetoothEffect::DiscoverNusService,
        ]))
    }

    pub fn services_discovered(
        &mut self,
        generation: CoreBluetoothGeneration,
        nus_service_present: bool,
    ) -> Result<CoreBluetoothApply, CoreBluetoothFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothPhase::DiscoveringServices {
            return Err(CoreBluetoothFailure::InvalidPhase);
        }
        if !nus_service_present {
            self.close();
            return Err(CoreBluetoothFailure::NusServiceMissing);
        }
        self.phase = CoreBluetoothPhase::DiscoveringCharacteristics;
        Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::DiscoverNusCharacteristics]))
    }

    pub fn characteristics_discovered(
        &mut self,
        generation: CoreBluetoothGeneration,
        characteristics: CoreBluetoothNusCharacteristics,
    ) -> Result<CoreBluetoothApply, CoreBluetoothFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothPhase::DiscoveringCharacteristics {
            return Err(CoreBluetoothFailure::InvalidPhase);
        }
        if !characteristics.write_characteristic_present {
            self.close();
            return Err(CoreBluetoothFailure::WriteCharacteristicMissing);
        }
        if !characteristics.notify_characteristic_present {
            self.close();
            return Err(CoreBluetoothFailure::NotifyCharacteristicMissing);
        }
        if !characteristics.write_with_response {
            self.close();
            return Err(CoreBluetoothFailure::WriteWithResponseMissing);
        }
        if !characteristics.notifications_supported {
            self.close();
            return Err(CoreBluetoothFailure::NotificationsUnsupported);
        }
        let write_limit = match BleWriteLimit::new(characteristics.maximum_write_value_length) {
            Ok(write_limit) => write_limit,
            Err(_) => {
                self.close();
                return Err(CoreBluetoothFailure::InvalidWriteLimit);
            }
        };
        self.write_limit = Some(write_limit);
        self.phase = CoreBluetoothPhase::EnablingNotifications;
        Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::EnableNotifications]))
    }

    pub fn notifications_changed(
        &mut self,
        generation: CoreBluetoothGeneration,
        enabled: bool,
    ) -> Result<CoreBluetoothApply, CoreBluetoothFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothPhase::EnablingNotifications {
            return Err(CoreBluetoothFailure::InvalidPhase);
        }
        if !enabled {
            self.close();
            return Err(CoreBluetoothFailure::NotificationSubscriptionFailed);
        }
        let write_limit = self.write_limit.ok_or(CoreBluetoothFailure::InvalidWriteLimit)?;
        self.phase = CoreBluetoothPhase::Ready;
        Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::Ready(write_limit)]))
    }

    pub fn notification(
        &self,
        generation: CoreBluetoothGeneration,
        bytes: Vec<u8>,
    ) -> Result<CoreBluetoothApply, CoreBluetoothFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothPhase::Ready {
            return Err(CoreBluetoothFailure::InvalidPhase);
        }
        Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::Notification(bytes)]))
    }

    pub fn begin_write(
        &mut self,
        bytes: Vec<u8>,
    ) -> Result<CoreBluetoothWriteRequest, CoreBluetoothFailure> {
        if self.phase != CoreBluetoothPhase::Ready {
            return Err(CoreBluetoothFailure::InvalidPhase);
        }
        if self.pending_write.is_some() {
            return Err(CoreBluetoothFailure::WriteInProgress);
        }
        let maximum = self.write_limit.ok_or(CoreBluetoothFailure::InvalidWriteLimit)?.bytes();
        if bytes.len() > maximum {
            return Err(CoreBluetoothFailure::WriteTooLarge { maximum, actual: bytes.len() });
        }
        let token = CoreBluetoothWriteToken(self.next_write);
        self.next_write =
            self.next_write.checked_add(1).ok_or(CoreBluetoothFailure::WriteFailed)?;
        self.pending_write = Some(token);
        Ok(CoreBluetoothWriteRequest { token, bytes })
    }

    pub fn write_completed(
        &mut self,
        generation: CoreBluetoothGeneration,
        token: CoreBluetoothWriteToken,
        succeeded: bool,
    ) -> Result<CoreBluetoothApply, CoreBluetoothFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothPhase::Ready || self.pending_write != Some(token) {
            return Err(CoreBluetoothFailure::WriteCallbackMismatch);
        }
        self.pending_write = None;
        if !succeeded {
            return Err(CoreBluetoothFailure::WriteFailed);
        }
        Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::WriteCompleted(token)]))
    }

    pub fn disconnected(&mut self, generation: CoreBluetoothGeneration) -> CoreBluetoothApply {
        if generation != self.generation {
            return CoreBluetoothApply::IgnoredStale;
        }
        if self.phase == CoreBluetoothPhase::Closed {
            self.disconnection_observed = true;
            return CoreBluetoothApply::Applied(Vec::new());
        }
        self.close();
        self.disconnection_observed = true;
        CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::Disconnected])
    }

    /// Mint proof that the generation-scoped NUS boundary is closed.
    ///
    /// # Errors
    ///
    /// Returns an error for stale generations or an active NUS boundary.
    pub fn shutdown_token(
        &mut self,
        generation: CoreBluetoothGeneration,
    ) -> Result<CoreBluetoothNusShutdown, CoreBluetoothFailure> {
        if generation != self.generation
            || self.phase != CoreBluetoothPhase::Closed
            || !self.disconnection_observed
            || self.shutdown_transferred
        {
            return Err(CoreBluetoothFailure::InvalidPhase);
        }
        self.shutdown_transferred = true;
        Ok(CoreBluetoothNusShutdown { generation })
    }

    pub fn close(&mut self) {
        self.phase = CoreBluetoothPhase::Closed;
        self.write_limit = None;
        self.pending_write = None;
        self.disconnection_observed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT: CoreBluetoothGeneration = CoreBluetoothGeneration::new(7);
    const STALE: CoreBluetoothGeneration = CoreBluetoothGeneration::new(6);

    fn valid_characteristics(limit: usize) -> CoreBluetoothNusCharacteristics {
        CoreBluetoothNusCharacteristics {
            write_characteristic_present: true,
            notify_characteristic_present: true,
            write_with_response: true,
            notifications_supported: true,
            maximum_write_value_length: limit,
        }
    }

    fn ready_boundary(limit: usize) -> CoreBluetoothAttemptBoundary {
        let mut boundary = CoreBluetoothAttemptBoundary::new(CURRENT);
        boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOn).unwrap();
        boundary.services_discovered(CURRENT, true).unwrap();
        boundary.characteristics_discovered(CURRENT, valid_characteristics(limit)).unwrap();
        boundary.notifications_changed(CURRENT, true).unwrap();
        boundary
    }

    #[test]
    fn valid_core_bluetooth_callbacks_publish_only_safe_rust_effects() {
        let mut boundary = CoreBluetoothAttemptBoundary::new(CURRENT);
        assert_eq!(
            boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOn),
            Ok(CoreBluetoothApply::Applied(vec![
                CoreBluetoothEffect::AdapterChanged(BleAdapterState::Ready),
                CoreBluetoothEffect::DiscoverNusService,
            ]))
        );
        assert_eq!(
            boundary.services_discovered(CURRENT, true),
            Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::DiscoverNusCharacteristics,]))
        );
        assert_eq!(
            boundary.characteristics_discovered(CURRENT, valid_characteristics(185)),
            Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::EnableNotifications,]))
        );
        assert_eq!(
            boundary.notifications_changed(CURRENT, true),
            Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::Ready(
                BleWriteLimit::new(185).unwrap()
            )]))
        );
        assert_eq!(boundary.phase(), CoreBluetoothPhase::Ready);
    }

    #[test]
    fn manager_service_and_characteristic_state_fail_closed() {
        let mut boundary = CoreBluetoothAttemptBoundary::new(CURRENT);
        assert_eq!(
            boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOff),
            Err(CoreBluetoothFailure::ManagerUnavailable(CoreBluetoothManagerState::PoweredOff))
        );
        assert_eq!(
            boundary.manager_changed(STALE, CoreBluetoothManagerState::PoweredOn),
            Ok(CoreBluetoothApply::IgnoredStale)
        );
        boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOn).unwrap();
        assert_eq!(
            boundary.services_discovered(CURRENT, false),
            Err(CoreBluetoothFailure::NusServiceMissing)
        );
        assert_eq!(boundary.phase(), CoreBluetoothPhase::Closed);

        for (properties, expected) in [
            (
                CoreBluetoothNusCharacteristics {
                    write_characteristic_present: false,
                    ..valid_characteristics(185)
                },
                CoreBluetoothFailure::WriteCharacteristicMissing,
            ),
            (
                CoreBluetoothNusCharacteristics {
                    notify_characteristic_present: false,
                    ..valid_characteristics(185)
                },
                CoreBluetoothFailure::NotifyCharacteristicMissing,
            ),
            (
                CoreBluetoothNusCharacteristics {
                    write_with_response: false,
                    ..valid_characteristics(185)
                },
                CoreBluetoothFailure::WriteWithResponseMissing,
            ),
            (
                CoreBluetoothNusCharacteristics {
                    notifications_supported: false,
                    ..valid_characteristics(185)
                },
                CoreBluetoothFailure::NotificationsUnsupported,
            ),
            (valid_characteristics(0), CoreBluetoothFailure::InvalidWriteLimit),
        ] {
            let mut boundary = CoreBluetoothAttemptBoundary::new(CURRENT);
            boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOn).unwrap();
            boundary.services_discovered(CURRENT, true).unwrap();
            assert_eq!(boundary.characteristics_discovered(CURRENT, properties), Err(expected));
            assert_eq!(boundary.phase(), CoreBluetoothPhase::Closed);
        }

        let mut boundary = CoreBluetoothAttemptBoundary::new(CURRENT);
        boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOn).unwrap();
        boundary.services_discovered(CURRENT, true).unwrap();
        boundary.characteristics_discovered(CURRENT, valid_characteristics(185)).unwrap();
        assert_eq!(
            boundary.notifications_changed(CURRENT, false),
            Err(CoreBluetoothFailure::NotificationSubscriptionFailed)
        );
        assert_eq!(boundary.phase(), CoreBluetoothPhase::Closed);
    }

    #[test]
    fn notification_bytes_remain_opaque_and_generation_scoped() {
        let boundary = ready_boundary(185);
        let fragmented_or_combined_kiss = vec![0xc0, 0x00, 0x01, 0xc0, 0xc0, 0x00];
        assert_eq!(
            boundary.notification(CURRENT, fragmented_or_combined_kiss.clone()),
            Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::Notification(
                fragmented_or_combined_kiss
            )]))
        );
        assert_eq!(boundary.notification(STALE, vec![0xff]), Ok(CoreBluetoothApply::IgnoredStale));
    }

    #[test]
    fn response_writes_are_bounded_serial_and_callback_scoped() {
        let mut boundary = ready_boundary(3);
        assert_eq!(
            boundary.begin_write(vec![1, 2, 3, 4]),
            Err(CoreBluetoothFailure::WriteTooLarge { maximum: 3, actual: 4 })
        );
        let write = boundary.begin_write(vec![1, 2, 3]).unwrap();
        assert_eq!(boundary.begin_write(vec![4]), Err(CoreBluetoothFailure::WriteInProgress));
        assert_eq!(
            boundary.write_completed(STALE, write.token, true),
            Ok(CoreBluetoothApply::IgnoredStale)
        );
        assert_eq!(
            boundary.write_completed(CURRENT, CoreBluetoothWriteToken(99), true),
            Err(CoreBluetoothFailure::WriteCallbackMismatch)
        );
        assert_eq!(
            boundary.write_completed(CURRENT, write.token, true),
            Ok(CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::WriteCompleted(write.token)]))
        );
        assert!(boundary.begin_write(vec![4]).is_ok());
    }

    #[test]
    fn disconnect_is_terminal_idempotent_and_rejects_late_callbacks() {
        let mut boundary = ready_boundary(20);
        assert_eq!(boundary.shutdown_token(CURRENT), Err(CoreBluetoothFailure::InvalidPhase));
        let write = boundary.begin_write(vec![1]).unwrap();
        assert_eq!(boundary.disconnected(STALE), CoreBluetoothApply::IgnoredStale);
        assert_eq!(
            boundary.disconnected(CURRENT),
            CoreBluetoothApply::Applied(vec![CoreBluetoothEffect::Disconnected])
        );
        assert_eq!(boundary.phase(), CoreBluetoothPhase::Closed);
        assert_eq!(
            boundary.write_completed(CURRENT, write.token, true),
            Err(CoreBluetoothFailure::WriteCallbackMismatch)
        );
        assert_eq!(boundary.disconnected(CURRENT), CoreBluetoothApply::Applied(Vec::new()));
        assert_eq!(boundary.phase(), CoreBluetoothPhase::Closed);
        assert_eq!(
            boundary.shutdown_token(CURRENT),
            Ok(CoreBluetoothNusShutdown { generation: CURRENT })
        );
        assert_eq!(boundary.shutdown_token(CURRENT), Err(CoreBluetoothFailure::InvalidPhase));
        assert_eq!(boundary.shutdown_token(STALE), Err(CoreBluetoothFailure::InvalidPhase));
        assert_eq!(
            boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOn),
            Err(CoreBluetoothFailure::InvalidPhase)
        );
    }
}
