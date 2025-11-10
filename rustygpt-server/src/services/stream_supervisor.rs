use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::{Duration, Instant},
};

use metrics::{counter, histogram};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStopReason {
    None,
    Cancelled,
    TimedOut,
    Completed,
}

const STATE_ACTIVE: u8 = 0;
const STATE_CANCELLED: u8 = 1;
const STATE_TIMEOUT: u8 = 2;
const STATE_COMPLETED: u8 = 3;

#[derive(Debug)]
pub struct StreamSession {
    token: CancellationToken,
    state: AtomicU8,
    started_at: Instant,
}

impl StreamSession {
    fn new(default_timeout: Option<Duration>) -> Arc<Self> {
        let session = Arc::new(Self {
            token: CancellationToken::new(),
            state: AtomicU8::new(STATE_ACTIVE),
            started_at: Instant::now(),
        });

        if let Some(duration) = default_timeout.filter(|d| !d.is_zero()) {
            let weak = Arc::downgrade(&session);
            tokio::spawn(async move {
                tokio::time::sleep(duration).await;
                if let Some(session) = weak.upgrade() {
                    session.mark_timeout();
                }
            });
        }

        session
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.clone()
    }

    pub fn mark_cancelled(&self) -> bool {
        if self
            .state
            .compare_exchange(
                STATE_ACTIVE,
                STATE_CANCELLED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            self.token.cancel();
            counter!("rustygpt_stream_cancels_total").increment(1);
            let elapsed_ms = self.started_at.elapsed().as_secs_f64() * 1000.0;
            histogram!("rustygpt_stream_cancel_latency_ms").record(elapsed_ms);
            true
        } else {
            false
        }
    }

    pub fn mark_timeout(&self) -> bool {
        if self
            .state
            .compare_exchange(
                STATE_ACTIVE,
                STATE_TIMEOUT,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            self.token.cancel();
            counter!("rustygpt_stream_timeouts_total").increment(1);
            true
        } else {
            false
        }
    }

    pub fn mark_completed(&self) {
        let _ = self.state.compare_exchange(
            STATE_ACTIVE,
            STATE_COMPLETED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }

    pub fn stop_reason(&self) -> StreamStopReason {
        match self.state.load(Ordering::SeqCst) {
            STATE_CANCELLED => StreamStopReason::Cancelled,
            STATE_TIMEOUT => StreamStopReason::TimedOut,
            STATE_COMPLETED => StreamStopReason::Completed,
            _ => StreamStopReason::None,
        }
    }
}

#[derive(Debug)]
pub struct StreamSupervisor {
    sessions: RwLock<HashMap<Uuid, Arc<StreamSession>>>,
    default_timeout: Option<Duration>,
}

impl StreamSupervisor {
    pub fn new(default_timeout: Option<Duration>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            default_timeout,
        }
    }

    pub fn create_session(&self) -> Arc<StreamSession> {
        StreamSession::new(self.default_timeout)
    }

    pub async fn register(&self, message_id: Uuid, session: Arc<StreamSession>) {
        let mut guard = self.sessions.write().await;
        guard.insert(message_id, session);
    }

    pub async fn unregister(&self, message_id: &Uuid) {
        let mut guard = self.sessions.write().await;
        guard.remove(message_id);
    }

    pub async fn cancel(&self, message_id: &Uuid) -> StreamStopReason {
        let session = {
            let guard = self.sessions.read().await;
            guard.get(message_id).cloned()
        };

        session.map_or(StreamStopReason::None, |session| {
            if session.mark_cancelled() {
                StreamStopReason::Cancelled
            } else {
                session.stop_reason()
            }
        })
    }
}

pub type SharedStreamSupervisor = Arc<StreamSupervisor>;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn cancel_idempotent_returns_same_reason() {
        let supervisor = StreamSupervisor::new(None);
        let session = supervisor.create_session();
        let message_id = Uuid::new_v4();
        supervisor.register(message_id, session).await;

        let first = supervisor.cancel(&message_id).await;
        let second = supervisor.cancel(&message_id).await;

        assert_eq!(first, StreamStopReason::Cancelled);
        assert_eq!(second, StreamStopReason::Cancelled);
    }

    #[tokio::test]
    async fn timeout_sets_stop_reason() {
        let supervisor = StreamSupervisor::new(Some(Duration::from_millis(20)));
        let session = supervisor.create_session();
        let message_id = Uuid::new_v4();
        supervisor.register(message_id, session).await;

        sleep(Duration::from_millis(40)).await;

        let reason = supervisor.cancel(&message_id).await;
        assert_eq!(reason, StreamStopReason::TimedOut);
    }
}
