//! Local browse observation budget; dropping a read does not cancel daemon work.
use std::future::Future;
use std::time::Duration;

pub const WAIT_SECONDS: u64 = 15;

pub async fn bounded<T>(read: impl Future<Output = Result<T, String>>) -> Result<T, String> {
    with_budget(read, Duration::from_secs(WAIT_SECONDS)).await
}

async fn with_budget<T>(
    read: impl Future<Output = Result<T, String>>,
    budget: Duration,
) -> Result<T, String> {
    tokio::time::timeout(budget, read).await.map_err(|_| "browse-local-deadline".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn completed_read_and_provider_failure_are_preserved() {
        assert_eq!(bounded(async { Ok(42) }).await, Ok(42));
        assert_eq!(bounded(async { Err::<(), _>("denied".into()) }).await, Err("denied".into()));
    }

    #[tokio::test]
    async fn deadline_drops_pending_read() {
        struct Dropped(std::sync::Arc<std::sync::atomic::AtomicBool>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let marker = Dropped(dropped.clone());
        let read = async move {
            let _marker = marker;
            std::future::pending::<Result<(), String>>().await
        };
        assert_eq!(with_budget(read, Duration::ZERO).await, Err("browse-local-deadline".into()));
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));
    }
}
