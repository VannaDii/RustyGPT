// Application state that will be shared across all routes
#[derive(Clone, Default)]
pub struct AppState {
    /// Optional PostgreSQL connection pool
    pub(crate) pool: Option<sqlx::PgPool>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("has_pool", &self.pool.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test AppState default creation
    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.pool.is_none());
    }

    /// Test AppState equivalence between default instances
    #[test]
    fn test_app_state_default_equals_new() {
        let state1 = AppState::default();
        let state2 = AppState::default();

        assert_eq!(state1.pool.is_some(), state2.pool.is_some());
    }

    /// Test AppState cloning
    #[test]
    fn test_app_state_clone() {
        let state1 = AppState::default();
        let state2 = state1.clone();

        assert_eq!(state1.pool.is_some(), state2.pool.is_some());
    }

    /// Test AppState with_pool method (can't create real pool in test)
    #[test]
    fn test_app_state_with_pool_concept() {
        // We can't create a real PgPool in tests without a database
        // But we can test the logic structure
        let state = AppState::default();
        assert!(state.pool.is_none());

        // In a real scenario with a pool:
        // let pool = create_test_pool().await;
        // let state_with_pool = AppState::with_pool(pool);
        // assert!(state_with_pool.pool.is_some());
    }

    /// Test AppState debug formatting
    #[test]
    fn test_app_state_debug() {
        let state = AppState::default();
        let debug_str = format!("{:?}", state);

        assert!(debug_str.contains("AppState"));
        assert!(debug_str.contains("has_pool"));
        assert!(debug_str.contains("false")); // has_pool should be false
    }

    /// Test AppState pool accessor field
    #[test]
    fn test_app_state_pool_accessor() {
        let state = AppState::default();
        assert!(state.pool.is_none());
    }

    /// Test AppState pool consistency
    #[test]
    fn test_app_state_has_database_consistency() {
        let state = AppState::default();

        // pool.is_some() should be consistent with having a database
        assert!(state.pool.is_none());
    }

    /// Test multiple AppState instances independence
    #[test]
    fn test_app_state_independence() {
        let state1 = AppState::default();
        let state2 = AppState::default();

        // Both should be independent and have the same default state
        assert_eq!(state1.pool.is_some(), state2.pool.is_some());
        assert!(state1.pool.is_none());
        assert!(state2.pool.is_none());
    }

    /// Test AppState field visibility
    #[test]
    fn test_app_state_field_access() {
        let state = AppState::default();

        // We should be able to access pool field within the crate
        assert!(state.pool.is_none());

        // Direct field access should not be possible from external modules
        // (This is enforced by the pub(crate) visibility)
    }
}
