use shared::models::SetupRequest;
use sqlx::{Error, PgPool};

use crate::auth::session::hash_password;

/// Checks if the database is already set up.
pub async fn is_setup(pool: &Option<PgPool>) -> Result<bool, Error> {
    // Check if database pool is available
    let pool_ref = pool.as_ref().ok_or(Error::PoolClosed)?;

    let configured = sqlx::query_scalar::<_, Option<bool>>("SELECT is_setup()")
        .fetch_one(pool_ref)
        .await?;

    Ok(configured.unwrap_or(false))
}

/// Performs the setup.
pub async fn init_setup(pool: &Option<PgPool>, config: &SetupRequest) -> Result<bool, Error> {
    // Check if the database is already set up
    if is_setup(pool).await? {
        return Ok(false);
    }

    // Check if database pool is available
    let pool_ref = pool.as_ref().ok_or(Error::PoolClosed)?;

    let password_hash =
        hash_password(&config.password).map_err(|err| Error::Protocol(err.to_string()))?;

    // Perform the setup
    let result = sqlx::query_scalar::<_, Option<bool>>("SELECT init_setup($1, $2, $3)")
        .bind(&config.username)
        .bind(&config.email)
        .bind(&password_hash)
        .fetch_one(pool_ref)
        .await?;

    Ok(result.unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::models::SetupRequest;
    use sqlx::PgPool;

    /// Test that is_setup returns error when database pool is None
    #[tokio::test]
    async fn test_is_setup_with_none_pool() {
        let pool: Option<PgPool> = None;

        // This should return an error when pool is None
        let result = is_setup(&pool).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::PoolClosed));
    }

    /// Test that init_setup returns error when database pool is None
    #[tokio::test]
    async fn test_init_setup_with_none_pool() {
        let pool: Option<PgPool> = None;
        let config = SetupRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "testpassword".to_string(),
        };

        // This should return an error when pool is None
        let result = init_setup(&pool, &config).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::PoolClosed));
    }

    /// Test SetupRequest creation and field access
    #[test]
    fn test_setup_request_creation() {
        let request = SetupRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "testpassword".to_string(),
        };

        assert_eq!(request.username, "testuser");
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "testpassword");
    }

    /// Test SetupRequest serialization to JSON
    #[test]
    fn test_setup_request_serialization() {
        let request = SetupRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "testpassword".to_string(),
        };

        let json = serde_json::to_string(&request).expect("Should serialize");
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
        assert!(json.contains("testpassword"));
    }

    /// Test SetupRequest deserialization from JSON
    #[test]
    fn test_setup_request_deserialization() {
        let json =
            r#"{"username":"testuser","email":"test@example.com","password":"testpassword"}"#;
        let request: SetupRequest = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(request.username, "testuser");
        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "testpassword");
    }

    /// Test SetupRequest with empty fields
    #[test]
    fn test_setup_request_with_empty_fields() {
        let request = SetupRequest {
            username: "".to_string(),
            email: "".to_string(),
            password: "".to_string(),
        };

        assert_eq!(request.username, "");
        assert_eq!(request.email, "");
        assert_eq!(request.password, "");
    }

    /// Test SetupRequest equality
    #[test]
    fn test_setup_request_equality() {
        let request1 = SetupRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "testpassword".to_string(),
        };

        let request2 = SetupRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "testpassword".to_string(),
        };

        assert_eq!(request1.username, request2.username);
        assert_eq!(request1.email, request2.email);
        assert_eq!(request1.password, request2.password);
    }
}
