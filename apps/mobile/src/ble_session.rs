use async_channel::Receiver;
use std::time::Duration;
use styrene_ui_platform::{BleRNodeByteAttempt, BleWriteLimit};
use styrened::mobile::{
    MobileBearerReason, MobileNode, MobileRNodeAttempt, MobileRNodeBearer, MobileRNodeByteStart,
    MobileRNodeWriteBatch, MobileRNodeWriteHandoff, RNodeBearerInfo, RNodeBearerKind,
};

struct PumpStart<A> {
    attempt: A,
    writes: Vec<Vec<u8>>,
}

struct PumpBatch<H> {
    handoff: H,
    writes: Vec<Vec<u8>>,
}

#[allow(async_fn_in_trait)]
trait RNodePumpSession {
    type Attempt: Copy;
    type Handoff: Copy;

    async fn start(&self, max_write_size: usize) -> Result<PumpStart<Self::Attempt>, String>;
    async fn submit(&self, attempt: Self::Attempt, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String>;
    async fn poll(
        &self,
        attempt: Self::Attempt,
    ) -> Result<Option<PumpBatch<Self::Handoff>>, String>;
    async fn complete(
        &self,
        attempt: Self::Attempt,
        handoff: Self::Handoff,
    ) -> Result<bool, String>;
    async fn fail(&self, attempt: Self::Attempt, handoff: Self::Handoff) -> Result<bool, String>;
    async fn stop(&self, attempt: Self::Attempt) -> Result<Vec<Vec<u8>>, String>;
}

struct MobileNodePump<'a>(&'a MobileNode);

impl RNodePumpSession for MobileNodePump<'_> {
    type Attempt = MobileRNodeAttempt;
    type Handoff = MobileRNodeWriteHandoff;

    async fn start(&self, max_write_size: usize) -> Result<PumpStart<Self::Attempt>, String> {
        let MobileRNodeByteStart { attempt, writes } = self
            .0
            .start_rnode_bytes(
                MobileRNodeBearer::BluetoothLe,
                RNodeBearerInfo {
                    kind: RNodeBearerKind::Ble,
                    negotiated_mtu: None,
                    max_write_size: Some(max_write_size),
                },
            )
            .await?;
        Ok(PumpStart { attempt, writes })
    }

    async fn submit(&self, attempt: Self::Attempt, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        self.0.submit_rnode_bytes(attempt, bytes).await
    }

    async fn poll(
        &self,
        attempt: Self::Attempt,
    ) -> Result<Option<PumpBatch<Self::Handoff>>, String> {
        Ok(self
            .0
            .poll_rnode_bytes(attempt)
            .await?
            .map(|MobileRNodeWriteBatch { handoff, writes }| PumpBatch { handoff, writes }))
    }

    async fn complete(
        &self,
        attempt: Self::Attempt,
        handoff: Self::Handoff,
    ) -> Result<bool, String> {
        self.0.complete_rnode_write(attempt, handoff).await
    }

    async fn fail(&self, attempt: Self::Attempt, handoff: Self::Handoff) -> Result<bool, String> {
        self.0.fail_rnode_write(attempt, handoff).await
    }

    async fn stop(&self, attempt: Self::Attempt) -> Result<Vec<Vec<u8>>, String> {
        self.0.stop_rnode_bytes(attempt, MobileBearerReason::ConnectionInterrupted).await
    }
}

/// Pump one connected BLE NUS attempt through the backend-owned `RNode` byte session.
///
/// # Errors
///
/// Returns a backend or platform failure. Outbound packets remain retained when
/// their complete write batch does not receive terminal success evidence.
pub async fn run_mobile_ble_rnode(
    node: &MobileNode,
    link: &mut impl BleRNodeByteAttempt,
    write_limit: BleWriteLimit,
    cancelled: &Receiver<()>,
) -> Result<(), String> {
    run_ble_pump(&MobileNodePump(node), link, write_limit.bytes(), cancelled).await
}

async fn run_ble_pump<S: RNodePumpSession>(
    session: &S,
    link: &mut impl BleRNodeByteAttempt,
    max_write_size: usize,
    cancelled: &Receiver<()>,
) -> Result<(), String> {
    let start = match session.start(max_write_size).await {
        Ok(start) => start,
        Err(error) => {
            link.close();
            return Err(error);
        }
    };
    let attempt = start.attempt;
    let result = async {
        if !write_all(link, start.writes, cancelled).await? {
            return Ok(());
        }
        loop {
            if let Some(batch) = session.poll(attempt).await? {
                match write_all(link, batch.writes, cancelled).await {
                    Ok(true) => {}
                    Ok(false) => return Ok(()),
                    Err(error) => {
                        let _ = session.fail(attempt, batch.handoff).await;
                        return Err(error);
                    }
                }
                if !session.complete(attempt, batch.handoff).await? {
                    return Err("RNode write handoff became stale before completion".into());
                }
            }
            let read = tokio::select! {
                _ = cancelled.recv() => return Ok(()),
                read = link.read() => read.map_err(|error| error.code)?,
                () = tokio::time::sleep(Duration::from_millis(20)) => continue,
            };
            if let Some(bytes) = read {
                let writes = session.submit(attempt, &bytes).await?;
                if !write_all(link, writes, cancelled).await? {
                    return Ok(());
                }
            }
        }
    }
    .await;

    // A cancelled response write may still be pending natively; never enqueue
    // shutdown bytes behind it on this interrupted attempt.
    let _ = session.stop(attempt).await;
    link.close();
    result
}

async fn write_all(
    link: &impl BleRNodeByteAttempt,
    writes: Vec<Vec<u8>>,
    cancelled: &Receiver<()>,
) -> Result<bool, String> {
    for write in writes {
        tokio::select! {
            _ = cancelled.recv() => return Ok(false),
            result = link.write_with_response(write) => {
                result.map_err(|error| error.code)?;
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::pending;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    use styrene_ui_platform::{PlatformFailure, PlatformFuture};

    use super::*;

    #[derive(Default)]
    struct FakeSessionState {
        starts: u64,
        submissions: Vec<Vec<u8>>,
        response_writes: VecDeque<Vec<Vec<u8>>>,
        retained: Option<PumpBatch<u64>>,
        offered_to: Option<u64>,
        completed: Vec<(u64, u64)>,
        failed: Vec<(u64, u64)>,
        stopped: Vec<u64>,
    }

    #[derive(Default)]
    struct FakeSession(Mutex<FakeSessionState>);

    impl RNodePumpSession for FakeSession {
        type Attempt = u64;
        type Handoff = u64;

        async fn start(&self, _max_write_size: usize) -> Result<PumpStart<u64>, String> {
            let mut state = self.0.lock().unwrap();
            state.starts += 1;
            Ok(PumpStart { attempt: state.starts, writes: vec![vec![0x01], vec![0x02]] })
        }

        async fn submit(&self, _attempt: u64, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
            let mut state = self.0.lock().unwrap();
            state.submissions.push(bytes.to_vec());
            Ok(state.response_writes.pop_front().unwrap_or_default())
        }

        async fn poll(&self, attempt: u64) -> Result<Option<PumpBatch<u64>>, String> {
            let mut state = self.0.lock().unwrap();
            if state.offered_to.is_some() {
                return Ok(None);
            }
            let Some(retained) = &state.retained else {
                return Ok(None);
            };
            let batch = PumpBatch { handoff: retained.handoff, writes: retained.writes.clone() };
            state.offered_to = Some(attempt);
            Ok(Some(batch))
        }

        async fn complete(&self, attempt: u64, handoff: u64) -> Result<bool, String> {
            let mut state = self.0.lock().unwrap();
            if state.offered_to != Some(attempt)
                || state.retained.as_ref().map(|batch| batch.handoff) != Some(handoff)
            {
                return Ok(false);
            }
            state.completed.push((attempt, handoff));
            state.retained = None;
            state.offered_to = None;
            Ok(true)
        }

        async fn fail(&self, attempt: u64, handoff: u64) -> Result<bool, String> {
            let mut state = self.0.lock().unwrap();
            if state.offered_to != Some(attempt)
                || state.retained.as_ref().map(|batch| batch.handoff) != Some(handoff)
            {
                return Ok(false);
            }
            state.failed.push((attempt, handoff));
            state.offered_to = None;
            Ok(true)
        }

        async fn stop(&self, attempt: u64) -> Result<Vec<Vec<u8>>, String> {
            let mut state = self.0.lock().unwrap();
            state.stopped.push(attempt);
            if state.offered_to == Some(attempt) {
                state.offered_to = None;
            }
            Ok(vec![vec![0xff]])
        }
    }

    enum FakeRead {
        Bytes(Vec<u8>),
        Empty,
        Disconnect,
        Block,
    }

    struct FakeLink {
        reads: Mutex<VecDeque<FakeRead>>,
        writes: Mutex<Vec<Vec<u8>>>,
        writing: Arc<AtomicBool>,
        blocked: Arc<AtomicBool>,
        write_count: Mutex<usize>,
        fail_at: Option<usize>,
        block_at: Option<usize>,
        closed: Arc<AtomicBool>,
    }

    impl FakeLink {
        fn new(
            reads: impl IntoIterator<Item = FakeRead>,
            fail_at: Option<usize>,
            block_at: Option<usize>,
        ) -> Self {
            Self {
                reads: Mutex::new(reads.into_iter().collect()),
                writes: Mutex::new(Vec::new()),
                writing: Arc::new(AtomicBool::new(false)),
                blocked: Arc::new(AtomicBool::new(false)),
                write_count: Mutex::new(0),
                fail_at,
                block_at,
                closed: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl BleRNodeByteAttempt for FakeLink {
        fn read(&self) -> PlatformFuture<'_, Result<Option<Vec<u8>>, PlatformFailure>> {
            Box::pin(async move {
                let read = { self.reads.lock().unwrap().pop_front().unwrap_or(FakeRead::Block) };
                match read {
                    FakeRead::Bytes(bytes) => Ok(Some(bytes)),
                    FakeRead::Empty => Ok(None),
                    FakeRead::Disconnect => {
                        Err(PlatformFailure { code: "ble_disconnected".into(), retryable: true })
                    }
                    FakeRead::Block => pending().await,
                }
            })
        }

        fn write_with_response(
            &self,
            data: Vec<u8>,
        ) -> PlatformFuture<'_, Result<(), PlatformFailure>> {
            Box::pin(async move {
                assert!(!self.writing.swap(true, Ordering::AcqRel));
                tokio::task::yield_now().await;
                let index = {
                    let mut count = self.write_count.lock().unwrap();
                    let index = *count;
                    *count += 1;
                    index
                };
                if self.block_at == Some(index) {
                    self.blocked.store(true, Ordering::Release);
                    pending::<()>().await;
                }
                if self.fail_at == Some(index) {
                    self.writing.store(false, Ordering::Release);
                    return Err(PlatformFailure {
                        code: "ble_write_failed".into(),
                        retryable: true,
                    });
                }
                self.writes.lock().unwrap().push(data);
                self.writing.store(false, Ordering::Release);
                Ok(())
            })
        }

        fn close(&mut self) {
            self.closed.store(true, Ordering::Release);
        }
    }

    #[tokio::test]
    async fn pump_preserves_notification_boundaries_and_serializes_response_writes() {
        let session = FakeSession::default();
        session.0.lock().unwrap().response_writes =
            VecDeque::from([vec![vec![0x11], vec![0x12]], vec![vec![0x21], vec![0x22]]]);
        let notifications = [vec![0xc0, 0x00], vec![0x01, 0xc0, 0xc0, 0x00, 0x02, 0xc0]];
        let mut link = FakeLink::new(
            [
                FakeRead::Bytes(notifications[0].clone()),
                FakeRead::Bytes(notifications[1].clone()),
                FakeRead::Disconnect,
            ],
            None,
            None,
        );
        let (_cancel, cancelled) = async_channel::bounded(1);

        assert_eq!(
            run_ble_pump(&session, &mut link, 20, &cancelled).await.unwrap_err(),
            "ble_disconnected"
        );
        let state = session.0.lock().unwrap();
        assert_eq!(state.submissions, notifications);
        assert_eq!(state.stopped, vec![1]);
        assert_eq!(
            *link.writes.lock().unwrap(),
            vec![vec![0x01], vec![0x02], vec![0x11], vec![0x12], vec![0x21], vec![0x22]]
        );
        assert!(link.closed.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn failed_handoff_replays_on_reconnect_and_cancellation_closes_attempt() {
        let session = FakeSession::default();
        session.0.lock().unwrap().retained =
            Some(PumpBatch { handoff: 7, writes: vec![vec![0x31], vec![0x32]] });
        let (_cancel, cancelled) = async_channel::bounded(1);
        let mut failed_link = FakeLink::new([FakeRead::Empty], Some(2), None);
        assert_eq!(
            run_ble_pump(&session, &mut failed_link, 20, &cancelled).await.unwrap_err(),
            "ble_write_failed"
        );
        assert_eq!(session.0.lock().unwrap().failed, vec![(1, 7)]);

        let mut replay_link = FakeLink::new([FakeRead::Empty, FakeRead::Disconnect], None, None);
        assert_eq!(
            run_ble_pump(&session, &mut replay_link, 20, &cancelled).await.unwrap_err(),
            "ble_disconnected"
        );
        assert_eq!(session.0.lock().unwrap().completed, vec![(2, 7)]);

        session.0.lock().unwrap().retained =
            Some(PumpBatch { handoff: 8, writes: vec![vec![0x41], vec![0x42]] });
        let (cancel, cancelled) = async_channel::bounded(1);
        let mut cancelled_link = FakeLink::new([FakeRead::Empty], None, Some(3));
        let blocked = Arc::clone(&cancelled_link.blocked);
        let run = run_ble_pump(&session, &mut cancelled_link, 20, &cancelled);
        let trigger = async move {
            while !blocked.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
            cancel.send(()).await.unwrap();
        };
        let (result, ()) = tokio::join!(run, trigger);
        result.unwrap();
        assert!(cancelled_link.closed.load(Ordering::Acquire));

        let (_cancel, cancelled) = async_channel::bounded(1);
        let mut reconnect_link = FakeLink::new([FakeRead::Empty, FakeRead::Disconnect], None, None);
        assert_eq!(
            run_ble_pump(&session, &mut reconnect_link, 20, &cancelled).await.unwrap_err(),
            "ble_disconnected"
        );
        let state = session.0.lock().unwrap();
        assert_eq!(state.starts, 4);
        assert_eq!(state.completed, vec![(2, 7), (4, 8)]);
        assert_eq!(state.stopped, vec![1, 2, 3, 4]);
    }
}
