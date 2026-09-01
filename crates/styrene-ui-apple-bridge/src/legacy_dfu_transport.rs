use styrene_ui_platform::{PlatformFailure, PlatformFuture};

use crate::{
    LegacyDfuAction, LegacyDfuFailure, LegacyDfuProgress, Rak4631LegacyDfuPlan,
    Rak4631LegacyDfuSession,
};

pub trait LegacyDfuGatt {
    fn write_control(&mut self, bytes: Vec<u8>) -> PlatformFuture<'_, Result<(), PlatformFailure>>;
    fn activate_and_reset(&mut self) -> PlatformFuture<'_, Result<(), PlatformFailure>>;
    fn write_packet(&mut self, bytes: Vec<u8>) -> PlatformFuture<'_, Result<(), PlatformFailure>>;
    fn notification(&mut self) -> PlatformFuture<'_, Result<Vec<u8>, PlatformFailure>>;
    fn remote_progress(&mut self, progress: LegacyDfuProgress) -> Result<(), PlatformFailure>;
    fn transfer_completed(&mut self) -> Result<(), PlatformFailure>;
    fn wait_disconnected(&mut self) -> PlatformFuture<'_, Result<(), PlatformFailure>>;
    fn close(&mut self);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LegacyDfuRunFailure {
    Protocol(LegacyDfuFailure),
    Transport(PlatformFailure),
}

/// Execute one admitted RAK4631 Legacy DFU plan over an already connected GATT link.
///
/// Progress is emitted only from bootloader notifications. Local packet enqueue
/// counts never become completion evidence.
///
/// # Errors
///
/// Returns a typed protocol or platform failure and closes the GATT attempt.
pub async fn run_rak4631_legacy_dfu(
    plan: Rak4631LegacyDfuPlan,
    observed_version: u16,
    gatt: &mut impl LegacyDfuGatt,
    mut progress: impl FnMut(LegacyDfuProgress),
) -> Result<(), LegacyDfuRunFailure> {
    struct CloseGuard<'a, G: LegacyDfuGatt + ?Sized>(&'a mut G);

    impl<G: LegacyDfuGatt + ?Sized> Drop for CloseGuard<'_, G> {
        fn drop(&mut self) {
            self.0.close();
        }
    }

    let gatt = CloseGuard(gatt);
    async {
        let mut session = Rak4631LegacyDfuSession::new(plan, observed_version)
            .map_err(LegacyDfuRunFailure::Protocol)?;
        loop {
            match session.next_action().map_err(LegacyDfuRunFailure::Protocol)? {
                LegacyDfuAction::WriteControl(bytes) => {
                    gatt.0.write_control(bytes).await.map_err(LegacyDfuRunFailure::Transport)?;
                }
                LegacyDfuAction::ActivateAndReset => {
                    gatt.0.activate_and_reset().await.map_err(LegacyDfuRunFailure::Transport)?;
                }
                LegacyDfuAction::WritePacket(bytes) => {
                    gatt.0.write_packet(bytes).await.map_err(LegacyDfuRunFailure::Transport)?;
                }
                LegacyDfuAction::AwaitNotification => {
                    let bytes =
                        gatt.0.notification().await.map_err(LegacyDfuRunFailure::Transport)?;
                    if let Some(update) =
                        session.notification(&bytes).map_err(LegacyDfuRunFailure::Protocol)?
                    {
                        gatt.0.remote_progress(update).map_err(LegacyDfuRunFailure::Transport)?;
                        progress(update);
                    }
                }
                LegacyDfuAction::TransferComplete => {
                    gatt.0.transfer_completed().map_err(LegacyDfuRunFailure::Transport)?;
                }
                LegacyDfuAction::AwaitDisconnect => {
                    gatt.0.wait_disconnected().await.map_err(LegacyDfuRunFailure::Transport)?;
                    session.disconnected().map_err(LegacyDfuRunFailure::Protocol)?;
                }
                LegacyDfuAction::Complete => return Ok(()),
            }
        }
    }
    .await
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use super::*;
    use crate::{RAK4631_DEVICE_TYPE, crc16_ccitt_false};

    #[derive(Default)]
    struct FakeGatt {
        controls: Vec<Vec<u8>>,
        activated: bool,
        packets: Vec<Vec<u8>>,
        notifications: VecDeque<Result<Vec<u8>, PlatformFailure>>,
        disconnected: bool,
        remote_progress: Vec<LegacyDfuProgress>,
        transfer_completed: bool,
        closed: bool,
        fail_packet: Option<usize>,
    }

    impl LegacyDfuGatt for FakeGatt {
        fn write_control(
            &mut self,
            bytes: Vec<u8>,
        ) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
            Box::pin(async move {
                self.controls.push(bytes);
                Ok(())
            })
        }

        fn activate_and_reset(&mut self) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
            Box::pin(async move {
                self.controls.push(vec![0x05]);
                self.activated = true;
                Ok(())
            })
        }

        fn write_packet(
            &mut self,
            bytes: Vec<u8>,
        ) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
            Box::pin(async move {
                if self.fail_packet == Some(self.packets.len()) {
                    return Err(PlatformFailure {
                        code: "synthetic_packet_failure".into(),
                        retryable: true,
                    });
                }
                self.packets.push(bytes);
                Ok(())
            })
        }

        fn notification(&mut self) -> PlatformFuture<'_, Result<Vec<u8>, PlatformFailure>> {
            Box::pin(async move {
                self.notifications.pop_front().unwrap_or_else(|| {
                    Err(PlatformFailure {
                        code: "synthetic_notification_missing".into(),
                        retryable: false,
                    })
                })
            })
        }

        fn remote_progress(&mut self, progress: LegacyDfuProgress) -> Result<(), PlatformFailure> {
            self.remote_progress.push(progress);
            Ok(())
        }

        fn transfer_completed(&mut self) -> Result<(), PlatformFailure> {
            self.transfer_completed = true;
            Ok(())
        }

        fn wait_disconnected(&mut self) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
            Box::pin(async move {
                self.disconnected = true;
                Ok(())
            })
        }

        fn close(&mut self) {
            self.closed = true;
        }
    }

    fn plan(length: usize) -> Rak4631LegacyDfuPlan {
        let application: Arc<[u8]> =
            (0..length).map(|value| value as u8).collect::<Vec<_>>().into();
        let mut init = Vec::new();
        init.extend_from_slice(&RAK4631_DEVICE_TYPE.to_le_bytes());
        init.extend_from_slice(&u16::MAX.to_le_bytes());
        init.extend_from_slice(&u32::MAX.to_le_bytes());
        init.extend_from_slice(&1u16.to_le_bytes());
        init.extend_from_slice(&0xFFFEu16.to_le_bytes());
        init.extend_from_slice(&crc16_ccitt_false(&application).to_le_bytes());
        Rak4631LegacyDfuPlan::new(init.into(), application).unwrap()
    }

    #[test]
    fn runner_serializes_the_complete_protocol_and_closes_after_activation() {
        let mut gatt = FakeGatt {
            notifications: VecDeque::from([
                Ok(vec![0x10, 0x01, 0x01]),
                Ok(vec![0x10, 0x02, 0x01]),
                Ok(vec![0x11, 160, 0, 0, 0]),
                Ok(vec![0x10, 0x03, 0x01]),
                Ok(vec![0x10, 0x04, 0x01]),
            ]),
            ..FakeGatt::default()
        };
        let mut progress = Vec::new();
        futures_lite::future::block_on(run_rak4631_legacy_dfu(
            plan(180),
            crate::RAK4631_LEGACY_DFU_VERSION,
            &mut gatt,
            |update| {
                progress.push(update);
            },
        ))
        .unwrap();

        assert_eq!(
            gatt.controls,
            vec![
                vec![0x01, 0x04],
                vec![0x02, 0x00],
                vec![0x02, 0x01],
                vec![0x08, 0x08, 0x00],
                vec![0x03],
                vec![0x04],
                vec![0x05],
            ]
        );
        assert!(gatt.packets.iter().all(|packet| packet.len() <= 20));
        assert!(gatt.disconnected);
        assert!(gatt.activated);
        assert!(gatt.transfer_completed);
        assert!(gatt.closed);
        assert_eq!(
            progress,
            vec![
                LegacyDfuProgress { completed: 160, total: 180 },
                LegacyDfuProgress { completed: 180, total: 180 },
            ]
        );
        assert_eq!(gatt.remote_progress, progress);
    }

    #[test]
    fn runner_closes_after_transport_failure_without_reporting_progress() {
        let mut gatt = FakeGatt { fail_packet: Some(0), ..FakeGatt::default() };
        let mut progress = Vec::new();
        let result = futures_lite::future::block_on(run_rak4631_legacy_dfu(
            plan(40),
            crate::RAK4631_LEGACY_DFU_VERSION,
            &mut gatt,
            |update| progress.push(update),
        ));
        assert!(matches!(result, Err(LegacyDfuRunFailure::Transport(_))));
        assert!(gatt.closed);
        assert!(progress.is_empty());
    }

    #[test]
    fn runner_closes_when_the_observed_bootloader_version_is_rejected() {
        let mut gatt = FakeGatt::default();
        let result = futures_lite::future::block_on(run_rak4631_legacy_dfu(
            plan(40),
            0x0007,
            &mut gatt,
            |_| {},
        ));
        assert_eq!(
            result,
            Err(LegacyDfuRunFailure::Protocol(LegacyDfuFailure::UnsupportedDfuVersion(0x0007)))
        );
        assert!(gatt.closed);
    }
}
