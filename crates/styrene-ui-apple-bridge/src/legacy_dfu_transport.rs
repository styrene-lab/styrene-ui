use styrene_ui_platform::{PlatformFailure, PlatformFuture};

use crate::{
    LegacyDfuAction, LegacyDfuFailure, LegacyDfuProgress, Rak4631LegacyDfuPlan,
    Rak4631LegacyDfuSession,
};

pub trait LegacyDfuGatt {
    fn write_control(&mut self, bytes: Vec<u8>) -> PlatformFuture<'_, Result<(), PlatformFailure>>;
    fn write_packet(&mut self, bytes: Vec<u8>) -> PlatformFuture<'_, Result<(), PlatformFailure>>;
    fn notification(&mut self) -> PlatformFuture<'_, Result<Vec<u8>, PlatformFailure>>;
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
    gatt: &mut impl LegacyDfuGatt,
    mut progress: impl FnMut(LegacyDfuProgress),
) -> Result<(), LegacyDfuRunFailure> {
    let mut session = Rak4631LegacyDfuSession::new(plan, crate::RAK4631_LEGACY_DFU_VERSION)
        .map_err(LegacyDfuRunFailure::Protocol)?;
    let result = async {
        loop {
            match session.next_action().map_err(LegacyDfuRunFailure::Protocol)? {
                LegacyDfuAction::WriteControl(bytes) => {
                    gatt.write_control(bytes).await.map_err(LegacyDfuRunFailure::Transport)?;
                }
                LegacyDfuAction::WritePacket(bytes) => {
                    gatt.write_packet(bytes).await.map_err(LegacyDfuRunFailure::Transport)?;
                }
                LegacyDfuAction::AwaitNotification => {
                    let bytes =
                        gatt.notification().await.map_err(LegacyDfuRunFailure::Transport)?;
                    if let Some(update) =
                        session.notification(&bytes).map_err(LegacyDfuRunFailure::Protocol)?
                    {
                        progress(update);
                    }
                }
                LegacyDfuAction::AwaitDisconnect => {
                    gatt.wait_disconnected().await.map_err(LegacyDfuRunFailure::Transport)?;
                    session.disconnected().map_err(LegacyDfuRunFailure::Protocol)?;
                }
                LegacyDfuAction::Complete => return Ok(()),
            }
        }
    }
    .await;
    gatt.close();
    result
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
        packets: Vec<Vec<u8>>,
        notifications: VecDeque<Result<Vec<u8>, PlatformFailure>>,
        disconnected: bool,
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
        futures_lite::future::block_on(run_rak4631_legacy_dfu(plan(180), &mut gatt, |update| {
            progress.push(update);
        }))
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
        assert!(gatt.closed);
        assert_eq!(
            progress,
            vec![
                LegacyDfuProgress { completed: 160, total: 180 },
                LegacyDfuProgress { completed: 180, total: 180 },
            ]
        );
    }

    #[test]
    fn runner_closes_after_transport_failure_without_reporting_progress() {
        let mut gatt = FakeGatt { fail_packet: Some(0), ..FakeGatt::default() };
        let mut progress = Vec::new();
        let result =
            futures_lite::future::block_on(run_rak4631_legacy_dfu(plan(40), &mut gatt, |update| {
                progress.push(update)
            }));
        assert!(matches!(result, Err(LegacyDfuRunFailure::Transport(_))));
        assert!(gatt.closed);
        assert!(progress.is_empty());
    }
}
