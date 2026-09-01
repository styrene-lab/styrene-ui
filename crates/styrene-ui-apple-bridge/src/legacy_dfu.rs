use std::sync::Arc;

pub const LEGACY_DFU_SERVICE_UUID: &str = "00001530-1212-EFDE-1523-785FEABCD123";
pub const LEGACY_DFU_CONTROL_POINT_UUID: &str = "00001531-1212-EFDE-1523-785FEABCD123";
pub const LEGACY_DFU_PACKET_UUID: &str = "00001532-1212-EFDE-1523-785FEABCD123";
pub const LEGACY_DFU_VERSION_UUID: &str = "00001534-1212-EFDE-1523-785FEABCD123";

pub const RAK4631_LEGACY_DFU_VERSION: u16 = 0x0008;
pub const RAK4631_DEVICE_TYPE: u16 = 0x0052;
pub const RAK4631_PACKET_BYTES: usize = 20;
pub const RAK4631_PRN_INTERVAL: u16 = 8;
pub const RAK4631_MAX_INIT_BYTES: usize = 64;
pub const RAK4631_MAX_APPLICATION_BYTES: usize = 0xF_4000 - 0x2_6000;

const START_DFU: u8 = 0x01;
const INITIALIZE_DFU: u8 = 0x02;
const RECEIVE_FIRMWARE: u8 = 0x03;
const VALIDATE_FIRMWARE: u8 = 0x04;
const SET_PRN: u8 = 0x08;
const RESPONSE: u8 = 0x10;
const PACKET_RECEIPT: u8 = 0x11;
const SUCCESS: u8 = 0x01;
const APPLICATION_IMAGE: u8 = 0x04;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rak4631LegacyDfuPlan {
    init_packet: Arc<[u8]>,
    application: Arc<[u8]>,
}

impl Rak4631LegacyDfuPlan {
    /// Validate already-admitted RAK4631 application bytes for Legacy DFU.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid size, alignment, target metadata, SoftDevice
    /// compatibility, or application CRC.
    pub fn new(init_packet: Arc<[u8]>, application: Arc<[u8]>) -> Result<Self, LegacyDfuFailure> {
        if application.is_empty() || application.len() > RAK4631_MAX_APPLICATION_BYTES {
            return Err(LegacyDfuFailure::InvalidApplicationSize);
        }
        if !application.len().is_multiple_of(4) {
            return Err(LegacyDfuFailure::ApplicationNotWordAligned);
        }
        if !(14..=RAK4631_MAX_INIT_BYTES).contains(&init_packet.len()) {
            return Err(LegacyDfuFailure::InvalidInitPacket);
        }
        let device_type = u16::from_le_bytes([init_packet[0], init_packet[1]]);
        if device_type != RAK4631_DEVICE_TYPE {
            return Err(LegacyDfuFailure::TargetMismatch);
        }
        let softdevice_count = usize::from(u16::from_le_bytes([init_packet[8], init_packet[9]]));
        let ids_end = 10usize
            .checked_add(
                softdevice_count.checked_mul(2).ok_or(LegacyDfuFailure::InvalidInitPacket)?,
            )
            .ok_or(LegacyDfuFailure::InvalidInitPacket)?;
        let crc_end = ids_end.checked_add(2).ok_or(LegacyDfuFailure::InvalidInitPacket)?;
        if softdevice_count == 0 || crc_end != init_packet.len() {
            return Err(LegacyDfuFailure::InvalidInitPacket);
        }
        let compatible = init_packet[10..ids_end]
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .any(|id| id == 0xFFFE);
        if !compatible {
            return Err(LegacyDfuFailure::SoftdeviceMismatch);
        }
        let declared_crc = u16::from_le_bytes([init_packet[ids_end], init_packet[ids_end + 1]]);
        if declared_crc != crc16_ccitt_false(&application) {
            return Err(LegacyDfuFailure::ApplicationCrcMismatch);
        }
        Ok(Self { init_packet, application })
    }

    #[must_use]
    pub fn application_len(&self) -> usize {
        self.application.len()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LegacyDfuAction {
    WriteControl(Vec<u8>),
    ActivateAndReset,
    WritePacket(Vec<u8>),
    AwaitNotification,
    TransferComplete,
    AwaitDisconnect,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LegacyDfuProgress {
    pub completed: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LegacyDfuFailure {
    InvalidApplicationSize,
    ApplicationNotWordAligned,
    InvalidInitPacket,
    TargetMismatch,
    SoftdeviceMismatch,
    ApplicationCrcMismatch,
    UnsupportedDfuVersion(u16),
    InvalidPhase,
    InvalidResponse,
    RemoteRejected { request: u8, status: u8 },
    InvalidRemoteOffset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Start,
    StartSize,
    StartResponse,
    InitStart,
    InitPackets,
    InitComplete,
    InitResponse,
    SetPrn,
    Receive,
    FirmwarePackets,
    FirmwareNotification,
    Validate,
    ValidateResponse,
    ReportComplete,
    Activate,
    Disconnect,
    Complete,
    Failed,
}

#[derive(Clone, Debug)]
pub struct Rak4631LegacyDfuSession {
    plan: Rak4631LegacyDfuPlan,
    phase: Phase,
    init_offset: usize,
    application_offset: usize,
    packets_since_prn: u16,
    remote_offset: usize,
}

impl Rak4631LegacyDfuSession {
    /// Start an exact RAK4631 Legacy DFU 0.8 session.
    ///
    /// # Errors
    ///
    /// Returns an error for an incompatible DFU Version characteristic.
    pub fn new(
        plan: Rak4631LegacyDfuPlan,
        observed_version: u16,
    ) -> Result<Self, LegacyDfuFailure> {
        if observed_version != RAK4631_LEGACY_DFU_VERSION {
            return Err(LegacyDfuFailure::UnsupportedDfuVersion(observed_version));
        }
        Ok(Self {
            plan,
            phase: Phase::Start,
            init_offset: 0,
            application_offset: 0,
            packets_since_prn: 0,
            remote_offset: 0,
        })
    }

    /// Return the next bounded transport action.
    ///
    /// # Errors
    ///
    /// Returns an error if the caller requests another action before reporting
    /// completion of the current notification or disconnect wait.
    pub fn next_action(&mut self) -> Result<LegacyDfuAction, LegacyDfuFailure> {
        match self.phase {
            Phase::Start => {
                self.phase = Phase::StartSize;
                Ok(LegacyDfuAction::WriteControl(vec![START_DFU, APPLICATION_IMAGE]))
            }
            Phase::StartSize => {
                let application_size = u32::try_from(self.plan.application.len())
                    .map_err(|_| LegacyDfuFailure::InvalidApplicationSize)?;
                let mut sizes = Vec::with_capacity(12);
                sizes.extend_from_slice(&0u32.to_le_bytes());
                sizes.extend_from_slice(&0u32.to_le_bytes());
                sizes.extend_from_slice(&application_size.to_le_bytes());
                self.phase = Phase::StartResponse;
                Ok(LegacyDfuAction::WritePacket(sizes))
            }
            Phase::StartResponse
            | Phase::InitResponse
            | Phase::FirmwareNotification
            | Phase::ValidateResponse => Ok(LegacyDfuAction::AwaitNotification),
            Phase::ReportComplete => {
                self.phase = Phase::Activate;
                Ok(LegacyDfuAction::TransferComplete)
            }
            Phase::InitStart => {
                self.phase = Phase::InitPackets;
                Ok(LegacyDfuAction::WriteControl(vec![INITIALIZE_DFU, 0x00]))
            }
            Phase::InitPackets => {
                if self.init_offset == self.plan.init_packet.len() {
                    self.phase = Phase::InitComplete;
                    return self.next_action();
                }
                let end = self
                    .init_offset
                    .saturating_add(RAK4631_PACKET_BYTES)
                    .min(self.plan.init_packet.len());
                let packet = self.plan.init_packet[self.init_offset..end].to_vec();
                self.init_offset = end;
                Ok(LegacyDfuAction::WritePacket(packet))
            }
            Phase::InitComplete => {
                self.phase = Phase::InitResponse;
                Ok(LegacyDfuAction::WriteControl(vec![INITIALIZE_DFU, 0x01]))
            }
            Phase::SetPrn => {
                self.phase = Phase::Receive;
                let [low, high] = RAK4631_PRN_INTERVAL.to_le_bytes();
                Ok(LegacyDfuAction::WriteControl(vec![SET_PRN, low, high]))
            }
            Phase::Receive => {
                self.phase = Phase::FirmwarePackets;
                Ok(LegacyDfuAction::WriteControl(vec![RECEIVE_FIRMWARE]))
            }
            Phase::FirmwarePackets => {
                if self.application_offset == self.plan.application.len() {
                    self.phase = Phase::FirmwareNotification;
                    return self.next_action();
                }
                let end = self
                    .application_offset
                    .saturating_add(RAK4631_PACKET_BYTES)
                    .min(self.plan.application.len());
                let packet = self.plan.application[self.application_offset..end].to_vec();
                self.application_offset = end;
                self.packets_since_prn += 1;
                if self.packets_since_prn == RAK4631_PRN_INTERVAL {
                    self.phase = Phase::FirmwareNotification;
                }
                Ok(LegacyDfuAction::WritePacket(packet))
            }
            Phase::Validate => {
                self.phase = Phase::ValidateResponse;
                Ok(LegacyDfuAction::WriteControl(vec![VALIDATE_FIRMWARE]))
            }
            Phase::Activate => {
                self.phase = Phase::Disconnect;
                Ok(LegacyDfuAction::ActivateAndReset)
            }
            Phase::Disconnect => Ok(LegacyDfuAction::AwaitDisconnect),
            Phase::Complete => Ok(LegacyDfuAction::Complete),
            Phase::Failed => Err(LegacyDfuFailure::InvalidPhase),
        }
    }

    /// Apply one control-point notification and return remote progress evidence.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed, unexpected, rejected, regressing, or
    /// out-of-range remote notifications.
    pub fn notification(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<LegacyDfuProgress>, LegacyDfuFailure> {
        let result = (|| match self.phase {
            Phase::StartResponse => {
                expect_response(bytes, START_DFU)?;
                self.phase = Phase::InitStart;
                Ok(None)
            }
            Phase::InitResponse => {
                expect_response(bytes, INITIALIZE_DFU)?;
                self.phase = Phase::SetPrn;
                Ok(None)
            }
            Phase::FirmwareNotification if bytes.first() == Some(&PACKET_RECEIPT) => {
                if bytes.len() != 5 {
                    return Err(LegacyDfuFailure::InvalidResponse);
                }
                let offset =
                    usize::try_from(u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]))
                        .map_err(|_| LegacyDfuFailure::InvalidRemoteOffset)?;
                if offset <= self.remote_offset
                    || offset != self.application_offset
                    || offset > self.plan.application.len()
                {
                    return Err(LegacyDfuFailure::InvalidRemoteOffset);
                }
                self.remote_offset = offset;
                self.packets_since_prn = 0;
                self.phase = Phase::FirmwarePackets;
                Ok(Some(self.progress()))
            }
            Phase::FirmwareNotification => {
                expect_response(bytes, RECEIVE_FIRMWARE)?;
                if self.application_offset != self.plan.application.len() {
                    return Err(LegacyDfuFailure::InvalidPhase);
                }
                self.remote_offset = self.plan.application.len();
                self.phase = Phase::Validate;
                Ok(Some(self.progress()))
            }
            Phase::ValidateResponse => {
                expect_response(bytes, VALIDATE_FIRMWARE)?;
                self.phase = Phase::ReportComplete;
                Ok(None)
            }
            _ => Err(LegacyDfuFailure::InvalidPhase),
        })();
        if result.is_err() {
            self.phase = Phase::Failed;
        }
        result
    }

    /// Record the expected disconnect after Activate and Reset.
    ///
    /// # Errors
    ///
    /// Returns an error unless remote validation succeeded and activation was sent.
    pub fn disconnected(&mut self) -> Result<(), LegacyDfuFailure> {
        if self.phase != Phase::Disconnect {
            return Err(LegacyDfuFailure::InvalidPhase);
        }
        self.phase = Phase::Complete;
        Ok(())
    }

    fn progress(&self) -> LegacyDfuProgress {
        LegacyDfuProgress {
            completed: self.remote_offset as u64,
            total: self.plan.application.len() as u64,
        }
    }
}

fn expect_response(bytes: &[u8], request: u8) -> Result<(), LegacyDfuFailure> {
    if bytes.len() != 3 || bytes[0] != RESPONSE || bytes[1] != request {
        return Err(LegacyDfuFailure::InvalidResponse);
    }
    if bytes[2] != SUCCESS {
        return Err(LegacyDfuFailure::RemoteRejected { request, status: bytes[2] });
    }
    Ok(())
}

#[must_use]
pub fn crc16_ccitt_false(bytes: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn application(length: usize) -> Arc<[u8]> {
        (0..length).map(|value| value as u8).collect::<Vec<_>>().into()
    }

    fn init_packet(application: &[u8]) -> Arc<[u8]> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&RAK4631_DEVICE_TYPE.to_le_bytes());
        bytes.extend_from_slice(&u16::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&0xFFFEu16.to_le_bytes());
        bytes.extend_from_slice(&crc16_ccitt_false(application).to_le_bytes());
        bytes.into()
    }

    fn plan(length: usize) -> Rak4631LegacyDfuPlan {
        let application = application(length);
        Rak4631LegacyDfuPlan::new(init_packet(&application), application).unwrap()
    }

    #[test]
    fn crc_matches_the_bootloader_check_vector() {
        assert_eq!(crc16_ccitt_false(b"123456789"), 0x29B1);
    }

    #[test]
    fn plan_rejects_wrong_target_crc_alignment_compatibility_and_size() {
        let app = application(40);
        let valid_init = init_packet(&app);

        let mut wrong_target = valid_init.to_vec();
        wrong_target[0] = 0x53;
        assert_eq!(
            Rak4631LegacyDfuPlan::new(wrong_target.into(), Arc::clone(&app)),
            Err(LegacyDfuFailure::TargetMismatch)
        );
        let mut wrong_crc = valid_init.to_vec();
        wrong_crc[12] ^= 1;
        assert_eq!(
            Rak4631LegacyDfuPlan::new(wrong_crc.into(), Arc::clone(&app)),
            Err(LegacyDfuFailure::ApplicationCrcMismatch)
        );
        let mut wrong_softdevice = valid_init.to_vec();
        wrong_softdevice[10..12].copy_from_slice(&1u16.to_le_bytes());
        assert_eq!(
            Rak4631LegacyDfuPlan::new(wrong_softdevice.into(), Arc::clone(&app)),
            Err(LegacyDfuFailure::SoftdeviceMismatch)
        );
        assert_eq!(
            Rak4631LegacyDfuPlan::new(valid_init, application(39)),
            Err(LegacyDfuFailure::ApplicationNotWordAligned)
        );
        assert_eq!(
            Rak4631LegacyDfuPlan::new(init_packet(&[]), Arc::from([])),
            Err(LegacyDfuFailure::InvalidApplicationSize)
        );
    }

    #[test]
    fn application_flow_uses_legacy_commands_fixed_packets_and_remote_progress() {
        let mut session =
            Rak4631LegacyDfuSession::new(plan(180), RAK4631_LEGACY_DFU_VERSION).unwrap();
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::WriteControl(vec![0x01, 0x04]));
        assert_eq!(
            session.next_action().unwrap(),
            LegacyDfuAction::WritePacket(vec![0, 0, 0, 0, 0, 0, 0, 0, 180, 0, 0, 0])
        );
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::AwaitNotification);
        session.notification(&[0x10, 0x01, 0x01]).unwrap();
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::WriteControl(vec![0x02, 0x00]));
        assert_eq!(
            session.next_action().unwrap(),
            LegacyDfuAction::WritePacket(session.plan.init_packet.to_vec())
        );
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::WriteControl(vec![0x02, 0x01]));
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::AwaitNotification);
        session.notification(&[0x10, 0x02, 0x01]).unwrap();
        assert_eq!(
            session.next_action().unwrap(),
            LegacyDfuAction::WriteControl(vec![0x08, 0x08, 0x00])
        );
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::WriteControl(vec![0x03]));

        for _ in 0..8 {
            assert!(matches!(
                session.next_action().unwrap(),
                LegacyDfuAction::WritePacket(packet) if packet.len() == 20
            ));
        }
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::AwaitNotification);
        assert_eq!(
            session.notification(&[0x11, 160, 0, 0, 0]).unwrap(),
            Some(LegacyDfuProgress { completed: 160, total: 180 })
        );
        assert!(matches!(
            session.next_action().unwrap(),
            LegacyDfuAction::WritePacket(packet) if packet.len() == 20
        ));
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::AwaitNotification);
        assert_eq!(
            session.notification(&[0x10, 0x03, 0x01]).unwrap(),
            Some(LegacyDfuProgress { completed: 180, total: 180 })
        );
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::WriteControl(vec![0x04]));
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::AwaitNotification);
        session.notification(&[0x10, 0x04, 0x01]).unwrap();
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::TransferComplete);
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::ActivateAndReset);
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::AwaitDisconnect);
        session.disconnected().unwrap();
        assert_eq!(session.next_action().unwrap(), LegacyDfuAction::Complete);
    }

    #[test]
    fn malformed_rejected_and_regressing_remote_evidence_fails_closed() {
        let mut session =
            Rak4631LegacyDfuSession::new(plan(160), RAK4631_LEGACY_DFU_VERSION).unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        assert_eq!(
            session.notification(&[0x10, 0x01, 0x05]),
            Err(LegacyDfuFailure::RemoteRejected { request: 0x01, status: 0x05 })
        );

        let mut session =
            Rak4631LegacyDfuSession::new(plan(160), RAK4631_LEGACY_DFU_VERSION).unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        session.notification(&[0x10, 0x01, 0x01]).unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        session.notification(&[0x10, 0x02, 0x01]).unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        for _ in 0..8 {
            session.next_action().unwrap();
        }
        assert_eq!(
            session.notification(&[0x11, 161, 0, 0, 0]),
            Err(LegacyDfuFailure::InvalidRemoteOffset)
        );
        assert_eq!(session.next_action(), Err(LegacyDfuFailure::InvalidPhase));
    }

    #[test]
    fn session_requires_legacy_dfu_zero_point_eight() {
        assert!(matches!(
            Rak4631LegacyDfuSession::new(plan(40), 0x0007),
            Err(LegacyDfuFailure::UnsupportedDfuVersion(0x0007))
        ));
    }

    #[test]
    fn underreported_prn_offset_is_terminal() {
        let mut session =
            Rak4631LegacyDfuSession::new(plan(160), RAK4631_LEGACY_DFU_VERSION).unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        session.notification(&[0x10, 0x01, 0x01]).unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        session.notification(&[0x10, 0x02, 0x01]).unwrap();
        session.next_action().unwrap();
        session.next_action().unwrap();
        for _ in 0..8 {
            session.next_action().unwrap();
        }
        assert_eq!(
            session.notification(&[0x11, 140, 0, 0, 0]),
            Err(LegacyDfuFailure::InvalidRemoteOffset)
        );
        assert_eq!(session.next_action(), Err(LegacyDfuFailure::InvalidPhase));
    }

    #[test]
    fn exact_and_short_final_packets_remain_word_aligned() {
        let mut exact =
            Rak4631LegacyDfuSession::new(plan(160), RAK4631_LEGACY_DFU_VERSION).unwrap();
        exact.next_action().unwrap();
        exact.next_action().unwrap();
        exact.notification(&[0x10, 0x01, 0x01]).unwrap();
        exact.next_action().unwrap();
        exact.next_action().unwrap();
        exact.next_action().unwrap();
        exact.notification(&[0x10, 0x02, 0x01]).unwrap();
        exact.next_action().unwrap();
        exact.next_action().unwrap();
        for _ in 0..8 {
            assert!(matches!(
                exact.next_action().unwrap(),
                LegacyDfuAction::WritePacket(packet) if packet.len() == 20
            ));
        }
        exact.notification(&[0x11, 160, 0, 0, 0]).unwrap();
        assert_eq!(exact.next_action().unwrap(), LegacyDfuAction::AwaitNotification);

        let mut short =
            Rak4631LegacyDfuSession::new(plan(164), RAK4631_LEGACY_DFU_VERSION).unwrap();
        short.next_action().unwrap();
        short.next_action().unwrap();
        short.notification(&[0x10, 0x01, 0x01]).unwrap();
        short.next_action().unwrap();
        short.next_action().unwrap();
        short.next_action().unwrap();
        short.notification(&[0x10, 0x02, 0x01]).unwrap();
        short.next_action().unwrap();
        short.next_action().unwrap();
        for _ in 0..8 {
            short.next_action().unwrap();
        }
        short.notification(&[0x11, 160, 0, 0, 0]).unwrap();
        assert!(matches!(
            short.next_action().unwrap(),
            LegacyDfuAction::WritePacket(packet) if packet.len() == 4
        ));
    }
}
