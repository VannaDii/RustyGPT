/// Trait for OAuth service functionality to enable testing with mocks
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait]
pub trait OAuthService {
    /// Handle Apple OAuth flow
    ///
    /// # Arguments
    /// * `pool` - Optional `PostgreSQL` connection pool
    /// * `auth_code` - Authorization code from Apple OAuth callback
    ///
    /// # Returns
    /// User ID if OAuth flow is successful
    ///
    /// # Errors
    /// Returns error if OAuth flow fails or database operations fail
    async fn handle_apple_oauth(
        &self,
        pool: Option<&PgPool>,
        auth_code: String,
    ) -> Result<Uuid, sqlx::Error>;

    /// Handle GitHub OAuth flow
    ///
    /// # Arguments
    /// * `pool` - Optional `PostgreSQL` connection pool
    /// * `auth_code` - Authorization code from GitHub OAuth callback
    ///
    /// # Returns
    /// User ID if OAuth flow is successful
    ///
    /// # Errors
    /// Returns error if OAuth flow fails or database operations fail
    async fn handle_github_oauth(
        &self,
        pool: Option<&PgPool>,
        auth_code: String,
    ) -> Result<Uuid, sqlx::Error>;
}

/// Production implementation of `OAuthService`
pub struct ProductionOAuthService;

#[async_trait]
impl OAuthService for ProductionOAuthService {
    async fn handle_apple_oauth(
        &self,
        pool: Option<&PgPool>,
        auth_code: String,
    ) -> Result<Uuid, sqlx::Error> {
        super::oauth_service::handle_apple_oauth(pool, auth_code).await
    }

    async fn handle_github_oauth(
        &self,
        pool: Option<&PgPool>,
        auth_code: String,
    ) -> Result<Uuid, sqlx::Error> {
        super::oauth_service::handle_github_oauth(pool, auth_code).await
    }
}

#[cfg(test)]
pub mod test_implementations {
    use super::*;

    /// Mock OAuth service for testing success scenarios
    pub struct MockOAuthServiceSuccess;

    #[async_trait]
    impl OAuthService for MockOAuthServiceSuccess {
        async fn handle_apple_oauth(
            &self,
            _pool: Option<&PgPool>,
            _auth_code: String,
        ) -> Result<Uuid, sqlx::Error> {
            Ok(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap())
        }

        async fn handle_github_oauth(
            &self,
            _pool: Option<&PgPool>,
            _auth_code: String,
        ) -> Result<Uuid, sqlx::Error> {
            Ok(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap())
        }
    }

    /// Mock OAuth service for testing failure scenarios
    pub struct MockOAuthServiceFailure;

    #[async_trait]
    impl OAuthService for MockOAuthServiceFailure {
        async fn handle_apple_oauth(
            &self,
            _pool: Option<&PgPool>,
            _auth_code: String,
        ) -> Result<Uuid, sqlx::Error> {
            Err(sqlx::Error::RowNotFound)
        }

        async fn handle_github_oauth(
            &self,
            _pool: Option<&PgPool>,
            _auth_code: String,
        ) -> Result<Uuid, sqlx::Error> {
            Err(sqlx::Error::RowNotFound)
        }
    }
}
