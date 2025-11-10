//! User service supporting the currently implemented OAuth flows.

use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

/// Minimal user service wrapper around stored procedures invoked by the OAuth handlers.
#[derive(Clone)]
pub struct UserService {
    pool: PgPool,
}

#[derive(Debug, Error)]
pub enum UserServiceError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("failed to register OAuth user")]
    MissingUserId,
}

type Result<T> = std::result::Result<T, UserServiceError>;

impl UserService {
    /// Construct a new service bound to the provided connection pool.
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a user via OAuth-specific stored procedure.
    pub async fn register_oauth_user(
        &self,
        username: &str,
        email: &str,
        apple_id: Option<&str>,
        github_id: Option<&str>,
    ) -> Result<Uuid> {
        let user_id: Option<Uuid> =
            sqlx::query_scalar("SELECT register_oauth_user($1, $2, $3, $4)")
                .bind(username)
                .bind(email)
                .bind(apple_id)
                .bind(github_id)
                .fetch_one(&self.pool)
                .await?;

        user_id.ok_or(UserServiceError::MissingUserId)
    }
}
