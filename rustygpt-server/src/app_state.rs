use std::sync::Arc;

use crate::{
    auth::session::SessionManager,
    middleware::rate_limit::RateLimitState,
    services::{
        assistant_service::AssistantRuntime, sse_persistence::SsePersistence,
        stream_supervisor::SharedStreamSupervisor,
    },
};

/// Application state shared across all routes.
#[derive(Clone, Default)]
pub struct AppState {
    /// Optional `PostgreSQL` connection pool
    pub(crate) pool: Option<sqlx::PgPool>,
    /// Assistant streaming service for automated replies
    pub(crate) assistant: Option<Arc<dyn AssistantRuntime>>,
    /// Optional persistence store for SSE history replay
    pub(crate) sse_store: Option<Arc<dyn SsePersistence>>,
    /// Session manager for cookie-backed authentication
    pub(crate) sessions: Option<Arc<dyn SessionManager>>,
    /// Dynamic rate limit manager
    pub(crate) rate_limits: Option<Arc<RateLimitState>>,
    /// Supervisor tracking in-flight assistant streams
    pub(crate) streams: Option<SharedStreamSupervisor>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("has_pool", &self.pool.is_some())
            .field("has_assistant", &self.assistant.is_some())
            .field("has_sse_store", &self.sse_store.is_some())
            .field("has_sessions", &self.sessions.is_some())
            .field("has_rate_limits", &self.rate_limits.is_some())
            .field("has_streams", &self.streams.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_empty() {
        let state = AppState::default();
        assert!(state.pool.is_none());
        assert!(state.assistant.is_none());
        assert!(state.sse_store.is_none());
        assert!(state.sessions.is_none());
        assert!(state.rate_limits.is_none());
        assert!(state.streams.is_none());
    }

    #[test]
    fn clone_preserves_flags() {
        let state1 = AppState::default();
        let state2 = state1.clone();
        assert_eq!(state1.pool.is_some(), state2.pool.is_some());
        assert_eq!(state1.assistant.is_some(), state2.assistant.is_some());
        assert_eq!(state1.sse_store.is_some(), state2.sse_store.is_some());
        assert_eq!(state1.sessions.is_some(), state2.sessions.is_some());
        assert_eq!(state1.rate_limits.is_some(), state2.rate_limits.is_some());
        assert_eq!(state1.streams.is_some(), state2.streams.is_some());
    }

    #[test]
    fn debug_lists_all_fields() {
        let state = AppState::default();
        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("has_pool"));
        assert!(debug_str.contains("has_assistant"));
        assert!(debug_str.contains("has_sse_store"));
        assert!(debug_str.contains("has_sessions"));
        assert!(debug_str.contains("has_rate_limits"));
        assert!(debug_str.contains("has_streams"));
    }

    #[test]
    fn independent_instances_match() {
        let a = AppState::default();
        let b = AppState::default();
        assert_eq!(a.pool.is_some(), b.pool.is_some());
        assert_eq!(a.assistant.is_some(), b.assistant.is_some());
        assert_eq!(a.sse_store.is_some(), b.sse_store.is_some());
        assert_eq!(a.sessions.is_some(), b.sessions.is_some());
        assert_eq!(a.rate_limits.is_some(), b.rate_limits.is_some());
        assert_eq!(a.streams.is_some(), b.streams.is_some());
    }
}
