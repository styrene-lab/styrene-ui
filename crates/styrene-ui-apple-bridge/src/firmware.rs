use styrene_ui_platform::BleWriteLimit;

use crate::{CoreBluetoothGeneration, CoreBluetoothNusShutdown};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreBluetoothDfuCharacteristics {
    pub control_point_present: bool,
    pub packet_present: bool,
    pub notifications_supported: bool,
    pub maximum_write_value_length: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothDfuPhase {
    Discovering,
    Ready,
    Writing,
    TransferComplete,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothDfuFailure {
    InvalidExpectedLength,
    InvalidPhase,
    DfuServiceMissing,
    ControlPointMissing,
    PacketCharacteristicMissing,
    NotificationsUnsupported,
    InvalidWriteLimit,
    InvalidProgress,
    ProgressRegressed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothDfuDisconnect {
    BeforeWrite,
    DuringWrite,
    AfterReportedTransfer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothDfuEffect {
    DiscoverDfuService,
    Ready(BleWriteLimit),
    Progress { completed: u64, total: u64 },
    TransferComplete,
    Disconnected(CoreBluetoothDfuDisconnect),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoreBluetoothDfuApply {
    Applied(Option<CoreBluetoothDfuEffect>),
    IgnoredStale,
}

/// Safe callback boundary for a future CoreBluetooth DFU implementation.
///
/// This type validates native observations only. It does not scan, connect, or
/// write to a peripheral.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreBluetoothDfuBoundary {
    generation: CoreBluetoothGeneration,
    phase: CoreBluetoothDfuPhase,
    expected_bytes: u64,
    completed_bytes: u64,
}

impl CoreBluetoothDfuBoundary {
    /// Create a DFU callback boundary after generation-scoped NUS shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error when the admitted application has no bytes.
    pub fn new(
        shutdown: CoreBluetoothNusShutdown,
        expected_bytes: u64,
    ) -> Result<Self, CoreBluetoothDfuFailure> {
        if expected_bytes == 0 {
            return Err(CoreBluetoothDfuFailure::InvalidExpectedLength);
        }
        Ok(Self {
            generation: shutdown.generation(),
            phase: CoreBluetoothDfuPhase::Discovering,
            expected_bytes,
            completed_bytes: 0,
        })
    }

    #[must_use]
    pub const fn phase(&self) -> CoreBluetoothDfuPhase {
        self.phase
    }

    #[must_use]
    pub const fn initial_effect(&self) -> CoreBluetoothDfuEffect {
        CoreBluetoothDfuEffect::DiscoverDfuService
    }

    /// Validate the discovered DFU service and required characteristics.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid phase or incomplete DFU capabilities.
    pub fn dfu_discovered(
        &mut self,
        generation: CoreBluetoothGeneration,
        service_present: bool,
        characteristics: CoreBluetoothDfuCharacteristics,
    ) -> Result<CoreBluetoothDfuApply, CoreBluetoothDfuFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothDfuApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothDfuPhase::Discovering {
            return Err(CoreBluetoothDfuFailure::InvalidPhase);
        }
        let failure = if !service_present {
            Some(CoreBluetoothDfuFailure::DfuServiceMissing)
        } else if !characteristics.control_point_present {
            Some(CoreBluetoothDfuFailure::ControlPointMissing)
        } else if !characteristics.packet_present {
            Some(CoreBluetoothDfuFailure::PacketCharacteristicMissing)
        } else if !characteristics.notifications_supported {
            Some(CoreBluetoothDfuFailure::NotificationsUnsupported)
        } else {
            None
        };
        if let Some(failure) = failure {
            self.phase = CoreBluetoothDfuPhase::Closed;
            return Err(failure);
        }
        let write_limit =
            BleWriteLimit::new(characteristics.maximum_write_value_length).map_err(|_| {
                self.phase = CoreBluetoothDfuPhase::Closed;
                CoreBluetoothDfuFailure::InvalidWriteLimit
            })?;
        self.phase = CoreBluetoothDfuPhase::Ready;
        Ok(CoreBluetoothDfuApply::Applied(Some(CoreBluetoothDfuEffect::Ready(write_limit))))
    }

    /// Record that the native transport crossed the write boundary.
    ///
    /// # Errors
    ///
    /// Returns an error unless DFU discovery completed successfully.
    pub fn write_started(
        &mut self,
        generation: CoreBluetoothGeneration,
    ) -> Result<CoreBluetoothDfuApply, CoreBluetoothDfuFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothDfuApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothDfuPhase::Ready {
            return Err(CoreBluetoothDfuFailure::InvalidPhase);
        }
        self.phase = CoreBluetoothDfuPhase::Writing;
        Ok(CoreBluetoothDfuApply::Applied(None))
    }

    /// Validate one generation-scoped native progress callback.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid phase, bounds, or regression.
    pub fn progress_changed(
        &mut self,
        generation: CoreBluetoothGeneration,
        completed: u64,
    ) -> Result<CoreBluetoothDfuApply, CoreBluetoothDfuFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothDfuApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothDfuPhase::Writing {
            return Err(CoreBluetoothDfuFailure::InvalidPhase);
        }
        if completed > self.expected_bytes {
            return Err(CoreBluetoothDfuFailure::InvalidProgress);
        }
        if completed < self.completed_bytes {
            return Err(CoreBluetoothDfuFailure::ProgressRegressed);
        }
        self.completed_bytes = completed;
        Ok(CoreBluetoothDfuApply::Applied(Some(CoreBluetoothDfuEffect::Progress {
            completed,
            total: self.expected_bytes,
        })))
    }

    /// Validate transfer completion after full progress.
    ///
    /// # Errors
    ///
    /// Returns an error unless the active write reached the admitted byte count.
    pub fn write_completed(
        &mut self,
        generation: CoreBluetoothGeneration,
    ) -> Result<CoreBluetoothDfuApply, CoreBluetoothDfuFailure> {
        if generation != self.generation {
            return Ok(CoreBluetoothDfuApply::IgnoredStale);
        }
        if self.phase != CoreBluetoothDfuPhase::Writing
            || self.completed_bytes != self.expected_bytes
        {
            return Err(CoreBluetoothDfuFailure::InvalidPhase);
        }
        self.phase = CoreBluetoothDfuPhase::TransferComplete;
        Ok(CoreBluetoothDfuApply::Applied(Some(CoreBluetoothDfuEffect::TransferComplete)))
    }

    pub fn disconnected(&mut self, generation: CoreBluetoothGeneration) -> CoreBluetoothDfuApply {
        if generation != self.generation {
            return CoreBluetoothDfuApply::IgnoredStale;
        }
        if self.phase == CoreBluetoothDfuPhase::Closed {
            return CoreBluetoothDfuApply::Applied(None);
        }
        let disposition = match self.phase {
            CoreBluetoothDfuPhase::Discovering | CoreBluetoothDfuPhase::Ready => {
                CoreBluetoothDfuDisconnect::BeforeWrite
            }
            CoreBluetoothDfuPhase::Writing => CoreBluetoothDfuDisconnect::DuringWrite,
            CoreBluetoothDfuPhase::TransferComplete => {
                CoreBluetoothDfuDisconnect::AfterReportedTransfer
            }
            CoreBluetoothDfuPhase::Closed => unreachable!(),
        };
        self.phase = CoreBluetoothDfuPhase::Closed;
        CoreBluetoothDfuApply::Applied(Some(CoreBluetoothDfuEffect::Disconnected(disposition)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CoreBluetoothApply, CoreBluetoothAttemptBoundary, CoreBluetoothManagerState,
        CoreBluetoothNusCharacteristics,
    };

    const CURRENT: CoreBluetoothGeneration = CoreBluetoothGeneration::new(7);
    const STALE: CoreBluetoothGeneration = CoreBluetoothGeneration::new(6);

    fn characteristics() -> CoreBluetoothDfuCharacteristics {
        CoreBluetoothDfuCharacteristics {
            control_point_present: true,
            packet_present: true,
            notifications_supported: true,
            maximum_write_value_length: 185,
        }
    }

    fn shutdown_nus() -> CoreBluetoothNusShutdown {
        let mut boundary = CoreBluetoothAttemptBoundary::new(CURRENT);
        boundary.manager_changed(CURRENT, CoreBluetoothManagerState::PoweredOn).unwrap();
        boundary.services_discovered(CURRENT, true).unwrap();
        boundary
            .characteristics_discovered(
                CURRENT,
                CoreBluetoothNusCharacteristics {
                    write_characteristic_present: true,
                    notify_characteristic_present: true,
                    write_with_response: true,
                    notifications_supported: true,
                    maximum_write_value_length: 185,
                },
            )
            .unwrap();
        boundary.notifications_changed(CURRENT, true).unwrap();
        assert_eq!(
            boundary.disconnected(CURRENT),
            CoreBluetoothApply::Applied(vec![crate::CoreBluetoothEffect::Disconnected,])
        );
        boundary.shutdown_token(CURRENT).unwrap()
    }

    fn ready_boundary() -> CoreBluetoothDfuBoundary {
        let mut boundary = CoreBluetoothDfuBoundary::new(shutdown_nus(), 10).unwrap();
        boundary.dfu_discovered(CURRENT, true, characteristics()).unwrap();
        boundary
    }

    #[test]
    fn dfu_boundary_requires_a_closed_nus_generation() {
        let mut active = CoreBluetoothAttemptBoundary::new(CURRENT);
        assert_eq!(active.shutdown_token(CURRENT), Err(crate::CoreBluetoothFailure::InvalidPhase));

        let boundary = CoreBluetoothDfuBoundary::new(shutdown_nus(), 10).unwrap();
        assert_eq!(boundary.phase(), CoreBluetoothDfuPhase::Discovering);
        assert_eq!(boundary.initial_effect(), CoreBluetoothDfuEffect::DiscoverDfuService);
    }

    #[test]
    fn dfu_discovery_fails_closed_on_incomplete_characteristics() {
        for (characteristics, expected) in [
            (
                CoreBluetoothDfuCharacteristics {
                    control_point_present: false,
                    ..characteristics()
                },
                CoreBluetoothDfuFailure::ControlPointMissing,
            ),
            (
                CoreBluetoothDfuCharacteristics { packet_present: false, ..characteristics() },
                CoreBluetoothDfuFailure::PacketCharacteristicMissing,
            ),
            (
                CoreBluetoothDfuCharacteristics {
                    notifications_supported: false,
                    ..characteristics()
                },
                CoreBluetoothDfuFailure::NotificationsUnsupported,
            ),
            (
                CoreBluetoothDfuCharacteristics {
                    maximum_write_value_length: 0,
                    ..characteristics()
                },
                CoreBluetoothDfuFailure::InvalidWriteLimit,
            ),
        ] {
            let mut boundary = CoreBluetoothDfuBoundary::new(shutdown_nus(), 10).unwrap();
            assert_eq!(boundary.dfu_discovered(CURRENT, true, characteristics), Err(expected));
            assert_eq!(boundary.phase(), CoreBluetoothDfuPhase::Closed);
        }
    }

    #[test]
    fn progress_and_completion_are_plan_bounded_and_generation_scoped() {
        let mut boundary = ready_boundary();
        boundary.write_started(CURRENT).unwrap();
        assert_eq!(
            boundary.progress_changed(STALE, u64::MAX),
            Ok(CoreBluetoothDfuApply::IgnoredStale)
        );
        assert_eq!(
            boundary.progress_changed(CURRENT, 11),
            Err(CoreBluetoothDfuFailure::InvalidProgress)
        );
        boundary.progress_changed(CURRENT, 5).unwrap();
        assert_eq!(
            boundary.progress_changed(CURRENT, 4),
            Err(CoreBluetoothDfuFailure::ProgressRegressed)
        );
        assert_eq!(boundary.write_completed(STALE), Ok(CoreBluetoothDfuApply::IgnoredStale));
        assert_eq!(boundary.phase(), CoreBluetoothDfuPhase::Writing);
        boundary.progress_changed(CURRENT, 10).unwrap();
        boundary.write_completed(CURRENT).unwrap();
        assert_eq!(boundary.phase(), CoreBluetoothDfuPhase::TransferComplete);
    }

    #[test]
    fn disconnect_preserves_the_phase_of_the_native_observation() {
        let mut before_write = ready_boundary();
        assert_eq!(
            before_write.disconnected(CURRENT),
            CoreBluetoothDfuApply::Applied(Some(CoreBluetoothDfuEffect::Disconnected(
                CoreBluetoothDfuDisconnect::BeforeWrite
            )))
        );

        let mut during_write = ready_boundary();
        during_write.write_started(CURRENT).unwrap();
        assert_eq!(
            during_write.disconnected(CURRENT),
            CoreBluetoothDfuApply::Applied(Some(CoreBluetoothDfuEffect::Disconnected(
                CoreBluetoothDfuDisconnect::DuringWrite
            )))
        );

        let mut after_transfer = ready_boundary();
        after_transfer.write_started(CURRENT).unwrap();
        after_transfer.progress_changed(CURRENT, 10).unwrap();
        after_transfer.write_completed(CURRENT).unwrap();
        assert_eq!(
            after_transfer.disconnected(CURRENT),
            CoreBluetoothDfuApply::Applied(Some(CoreBluetoothDfuEffect::Disconnected(
                CoreBluetoothDfuDisconnect::AfterReportedTransfer
            )))
        );
    }
}
