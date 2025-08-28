/// User service for authentication and user management
use anyhow::Result;
use shared::models::User;
use sqlx::PgPool;
use uuid::Uuid;

/// Service for handling user-related database operations
pub struct UserService {
    pool: PgPool,
}

impl UserService {
    /// Creates a new [`UserService`] instance
    ///
    /// # Arguments
    /// * `pool` - A [`PgPool`] database connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Registers a new user with username, email and password
    ///
    /// # Arguments
    /// * `username` - User's chosen username
    /// * `email` - User's email address
    /// * `password_hash` - Hashed password
    ///
    /// # Returns
    /// Returns the new user's [`Uuid`] on success
    ///
    /// # Errors
    /// Returns an error if the username or email already exists or database operation fails
    pub async fn register_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
    ) -> Result<Uuid> {
        let user_id: Option<Uuid> = sqlx::query_scalar("SELECT register_user($1, $2, $3)")
            .bind(username)
            .bind(email)
            .bind(password_hash)
            .fetch_one(&self.pool)
            .await?;

        user_id.ok_or_else(|| anyhow::anyhow!("Failed to register user"))
    }

    /// Authenticates a user by username or email and returns user data
    ///
    /// # Arguments
    /// * `username_or_email` - User's username or email address
    ///
    /// # Returns
    /// Returns `Some((user_id, username, email, password_hash))` if user exists, `None` otherwise
    ///
    /// # Errors
    /// Returns an error if database operation fails
    pub async fn authenticate_user_unified(
        &self,
        username_or_email: &str,
    ) -> Result<Option<(Uuid, String, String, String)>> {
        let result = sqlx::query_as::<_, (Uuid, String, String, String)>(
            "SELECT id, username, email, password_hash FROM authenticate_user_unified($1)",
        )
        .bind(username_or_email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Registers or updates a user via OAuth
    ///
    /// # Arguments
    /// * `username` - User's chosen username
    /// * `email` - User's email
    /// * `apple_id` - Apple OAuth ID (optional)
    /// * `github_id` - GitHub OAuth ID (optional)
    ///
    /// # Returns
    /// Returns the user's [`Uuid`] on success
    ///
    /// # Errors
    /// Returns an error if database operation fails
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

        user_id.ok_or_else(|| anyhow::anyhow!("Failed to register OAuth user"))
    }

    /// Gets user details by ID
    ///
    /// # Arguments
    /// * `user_id` - The user's [`Uuid`]
    ///
    /// # Returns
    /// Returns a [`User`] if found
    ///
    /// # Errors
    /// Returns an error if user not found or database operation fails
    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<Option<User>> {
        let result = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<String>,
                Option<String>,
                chrono::NaiveDateTime,
            ),
        >(
            "SELECT id, username, email, apple_id, github_id, created_at FROM get_user_by_id($1)",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(
            |(id, username, email, _apple_id, _github_id, created_at)| User {
                id,
                username,
                email,
                created_at: shared::models::Timestamp(chrono::DateTime::from_naive_utc_and_offset(
                    created_at,
                    chrono::Utc,
                )),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    async fn setup_test_db() -> PgPool {
        // This would be a test database setup in real implementation
        todo!("Setup test database")
    }

    #[tokio::test]
    #[ignore = "Requires test database setup"]
    async fn test_register_user() {
        let pool = setup_test_db().await;
        let service = UserService::new(pool);

        let user_id = service
            .register_user("testuser", "test@example.com", "hashedpassword")
            .await
            .unwrap();

        assert!(!user_id.is_nil());
    }

    #[tokio::test]
    #[ignore = "Requires test database setup"]
    async fn test_authenticate_user_unified() {
        let pool = setup_test_db().await;
        let service = UserService::new(pool);

        // First register a user
        let _ = service
            .register_user("testuser", "test@example.com", "hashedpassword")
            .await
            .unwrap();

        // Test authenticate by email
        let result = service
            .authenticate_user_unified("test@example.com")
            .await
            .unwrap();
        assert!(result.is_some());
        let (user_id, username, email, password_hash) = result.unwrap();
        assert!(!user_id.is_nil());
        assert_eq!(username, "testuser");
        assert_eq!(email, "test@example.com");
        assert_eq!(password_hash, "hashedpassword");

        // Test authenticate by username
        let result = service.authenticate_user_unified("testuser").await.unwrap();
        assert!(result.is_some());
        let (user_id, username, email, password_hash) = result.unwrap();
        assert!(!user_id.is_nil());
        assert_eq!(username, "testuser");
        assert_eq!(email, "test@example.com");
        assert_eq!(password_hash, "hashedpassword");
    }
}
