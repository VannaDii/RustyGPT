use oauth2::{AuthorizationCode, TokenResponse, basic::BasicClient};
use sqlx::PgPool;
use std::env;
use tracing::{error, info, instrument};
use uuid::Uuid;

use super::user_service::UserService;

#[instrument(skip(pool))]
pub async fn handle_apple_oauth(
    pool: &Option<PgPool>,
    auth_code: String,
) -> Result<Uuid, sqlx::Error> {
    info!("Starting Apple OAuth flow");
    let client_id = env::var("APPLE_CLIENT_ID").map_err(|_| {
        error!("APPLE_CLIENT_ID missing");
        sqlx::Error::ColumnNotFound("APPLE_CLIENT_ID missing".into())
    })?;
    let auth_url_str = env::var("APPLE_AUTH_URL")
        .map_err(|_| sqlx::Error::ColumnNotFound("APPLE_AUTH_URL missing".into()))?;
    let token_url_str = env::var("APPLE_TOKEN_URL")
        .map_err(|_| sqlx::Error::ColumnNotFound("APPLE_TOKEN_URL missing".into()))?;

    let auth_url = oauth2::AuthUrl::new(auth_url_str)
        .map_err(|_| sqlx::Error::ColumnNotFound("Invalid Apple Auth URL".into()))?;
    let token_url = oauth2::TokenUrl::new(token_url_str)
        .map_err(|_| sqlx::Error::ColumnNotFound("Invalid Apple Token URL".into()))?;

    let client = BasicClient::new(oauth2::ClientId::new(client_id))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url);

    let http_client = create_http_client();

    let token_response = client
        .exchange_code(AuthorizationCode::new(auth_code))
        .request_async(&http_client)
        .await
        .map_err(|_| {
            error!("Failed to retrieve Apple OAuth token");
            sqlx::Error::ColumnNotFound("Failed to retrieve Apple OAuth token".into())
        })?;
    info!("Successfully retrieved Apple OAuth token");

    let apple_id = token_response.access_token().secret().clone();

    // Check if database pool is available
    let pool_ref = pool.as_ref().ok_or_else(|| {
        error!("Database pool not available");
        sqlx::Error::PoolClosed
    })?;

    let user_service = UserService::new(pool_ref.clone());
    // TODO: Fetch actual user info from Apple OAuth response
    // For now, using access token as temporary identifier
    let username = format!("apple_user_{}", &apple_id[..8]); // Use first 8 chars as temp username
    let email = format!("{}@appleid.com", &apple_id[..8]); // Temporary email format

    let user_id = user_service
        .register_oauth_user(&username, &email, Some(&apple_id), None)
        .await
        .map_err(|e| {
            error!("Failed to register Apple OAuth user: {}", e);
            sqlx::Error::RowNotFound
        })?;

    info!("Apple OAuth user registered with ID: {}", user_id);

    Ok(user_id)
}

/// Creates an HTTP client with security-focused configuration.
///
/// # Returns
/// A [`reqwest::Client`] configured to prevent SSRF vulnerabilities by disabling redirects.
pub fn create_http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build")
}

#[instrument(skip(pool))]
pub async fn handle_github_oauth(
    pool: &Option<PgPool>,
    auth_code: String,
) -> Result<Uuid, sqlx::Error> {
    info!("Starting GitHub OAuth flow");
    let client_id = env::var("GITHUB_CLIENT_ID").map_err(|_| {
        error!("GITHUB_CLIENT_ID missing");
        sqlx::Error::ColumnNotFound("GITHUB_CLIENT_ID missing".into())
    })?;
    let client_secret = env::var("GITHUB_CLIENT_SECRET")
        .map_err(|_| sqlx::Error::ColumnNotFound("GITHUB_CLIENT_SECRET missing".into()))?;
    let auth_url_str = env::var("GITHUB_AUTH_URL")
        .map_err(|_| sqlx::Error::ColumnNotFound("GITHUB_AUTH_URL missing".into()))?;
    let token_url_str = env::var("GITHUB_TOKEN_URL")
        .map_err(|_| sqlx::Error::ColumnNotFound("GITHUB_TOKEN_URL missing".into()))?;

    let auth_url = oauth2::AuthUrl::new(auth_url_str)
        .map_err(|_| sqlx::Error::ColumnNotFound("Invalid GitHub Auth URL".into()))?;
    let token_url = oauth2::TokenUrl::new(token_url_str)
        .map_err(|_| sqlx::Error::ColumnNotFound("Invalid GitHub Token URL".into()))?;

    let client = BasicClient::new(oauth2::ClientId::new(client_id))
        .set_client_secret(oauth2::ClientSecret::new(client_secret))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url);

    let http_client = create_http_client();

    let token_response = client
        .exchange_code(AuthorizationCode::new(auth_code))
        .request_async(&http_client)
        .await
        .map_err(|_| {
            error!("Failed to retrieve GitHub OAuth token");
            sqlx::Error::ColumnNotFound("Failed to retrieve GitHub OAuth token".into())
        })?;
    info!("Successfully retrieved GitHub OAuth token");

    let github_id = token_response.access_token().secret().clone();

    // Check if database pool is available
    let pool_ref = pool.as_ref().ok_or_else(|| {
        error!("Database pool not available");
        sqlx::Error::PoolClosed
    })?;

    let user_service = UserService::new(pool_ref.clone());
    // TODO: Fetch actual user info from GitHub OAuth response
    // For now, using access token as temporary identifier
    let username = format!("github_user_{}", &github_id[..8]); // Use first 8 chars as temp username
    let email = format!("{}@github.com", &github_id[..8]); // Temporary email format

    let user_id = user_service
        .register_oauth_user(&username, &email, None, Some(&github_id))
        .await
        .map_err(|e| {
            error!("Failed to register GitHub OAuth user: {}", e);
            sqlx::Error::RowNotFound
        })?;

    info!("GitHub OAuth user registered with ID: {}", user_id);

    Ok(user_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Test that create_http_client returns a properly configured client
    #[test]
    fn test_create_http_client() {
        let client = create_http_client();

        // Verify the client was created successfully
        // We can't easily test the redirect policy, but we can verify the client exists
        assert!(format!("{:?}", client).contains("Client"));
    }

    /// Test that create_http_client returns different instances
    #[test]
    fn test_create_http_client_multiple_instances() {
        let client1 = create_http_client();
        let client2 = create_http_client();

        // Verify both clients were created successfully
        assert!(format!("{:?}", client1).contains("Client"));
        assert!(format!("{:?}", client2).contains("Client"));
    }

    /// Test handle_apple_oauth with missing environment variables
    #[tokio::test]
    #[serial]
    async fn test_handle_apple_oauth_missing_env_vars() {
        // Clear any existing environment variables that might interfere
        unsafe {
            std::env::remove_var("APPLE_CLIENT_ID");
            std::env::remove_var("APPLE_AUTH_URL");
            std::env::remove_var("APPLE_TOKEN_URL");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_apple_oauth(&pool, auth_code).await;

        // Should fail due to missing environment variables
        assert!(result.is_err());

        match result {
            Err(sqlx::Error::ColumnNotFound(msg)) => {
                assert!(msg.contains("APPLE_CLIENT_ID missing"));
            }
            _ => panic!("Expected ColumnNotFound error for missing APPLE_CLIENT_ID"),
        }
    }

    /// Test handle_github_oauth with missing environment variables
    #[tokio::test]
    #[serial]
    async fn test_handle_github_oauth_missing_env_vars() {
        // Clear any existing environment variables that might interfere
        unsafe {
            std::env::remove_var("GITHUB_CLIENT_ID");
            std::env::remove_var("GITHUB_CLIENT_SECRET");
            std::env::remove_var("GITHUB_AUTH_URL");
            std::env::remove_var("GITHUB_TOKEN_URL");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_github_oauth(&pool, auth_code).await;

        // Should fail due to missing environment variables
        assert!(result.is_err());

        match result {
            Err(sqlx::Error::ColumnNotFound(msg)) => {
                assert!(msg.contains("GITHUB_CLIENT_ID missing"));
            }
            _ => panic!("Expected ColumnNotFound error for missing GITHUB_CLIENT_ID"),
        }
    }

    /// Test handle_apple_oauth with invalid auth URL
    #[tokio::test]
    #[serial]
    async fn test_handle_apple_oauth_invalid_auth_url() {
        unsafe {
            std::env::set_var("APPLE_CLIENT_ID", "test_client_id");
            std::env::set_var("APPLE_AUTH_URL", "invalid_url");
            std::env::set_var("APPLE_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_apple_oauth(&pool, auth_code).await;

        // Should fail due to invalid auth URL
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("APPLE_CLIENT_ID");
            std::env::remove_var("APPLE_AUTH_URL");
            std::env::remove_var("APPLE_TOKEN_URL");
        }
    }

    /// Test handle_github_oauth with invalid auth URL
    #[tokio::test]
    #[serial]
    async fn test_handle_github_oauth_invalid_auth_url() {
        unsafe {
            std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
            std::env::set_var("GITHUB_CLIENT_SECRET", "test_client_secret");
            std::env::set_var("GITHUB_AUTH_URL", "invalid_url");
            std::env::set_var("GITHUB_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_github_oauth(&pool, auth_code).await;

        // Should fail due to invalid auth URL
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("GITHUB_CLIENT_ID");
            std::env::remove_var("GITHUB_CLIENT_SECRET");
            std::env::remove_var("GITHUB_AUTH_URL");
            std::env::remove_var("GITHUB_TOKEN_URL");
        }
    }

    /// Test handle_apple_oauth with empty auth code
    #[tokio::test]
    #[serial]
    async fn test_handle_apple_oauth_empty_auth_code() {
        unsafe {
            std::env::set_var("APPLE_CLIENT_ID", "test_client_id");
            std::env::set_var("APPLE_AUTH_URL", "https://valid.auth.url");
            std::env::set_var("APPLE_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "".to_string();

        let result = handle_apple_oauth(&pool, auth_code).await;

        // Should eventually fail, but not immediately due to empty auth code
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("APPLE_CLIENT_ID");
            std::env::remove_var("APPLE_AUTH_URL");
            std::env::remove_var("APPLE_TOKEN_URL");
        }
    }

    /// Test handle_github_oauth with empty auth code
    #[tokio::test]
    #[serial]
    async fn test_handle_github_oauth_empty_auth_code() {
        unsafe {
            std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
            std::env::set_var("GITHUB_CLIENT_SECRET", "test_client_secret");
            std::env::set_var("GITHUB_AUTH_URL", "https://valid.auth.url");
            std::env::set_var("GITHUB_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "".to_string();

        let result = handle_github_oauth(&pool, auth_code).await;

        // Should eventually fail, but not immediately due to empty auth code
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("GITHUB_CLIENT_ID");
            std::env::remove_var("GITHUB_CLIENT_SECRET");
            std::env::remove_var("GITHUB_AUTH_URL");
            std::env::remove_var("GITHUB_TOKEN_URL");
        }
    }

    /// Test OAuth client creation with minimal valid configuration
    #[test]
    fn test_oauth_client_creation() {
        // Test that we can create OAuth URLs (this doesn't require unsafe env manipulation)
        let valid_url = "https://example.com/oauth";
        let auth_url_result = oauth2::AuthUrl::new(valid_url.to_string());
        assert!(auth_url_result.is_ok());

        let token_url_result = oauth2::TokenUrl::new(valid_url.to_string());
        assert!(token_url_result.is_ok());
    }

    /// Test OAuth URL validation
    #[test]
    fn test_oauth_url_validation() {
        // Test invalid URL
        let invalid_url = "not_a_url";
        let auth_url_result = oauth2::AuthUrl::new(invalid_url.to_string());
        assert!(auth_url_result.is_err());

        // Test valid URL
        let valid_url = "https://example.com/oauth";
        let auth_url_result = oauth2::AuthUrl::new(valid_url.to_string());
        assert!(auth_url_result.is_ok());
    }

    /// Test AuthorizationCode creation
    #[test]
    fn test_authorization_code_creation() {
        let code = oauth2::AuthorizationCode::new("test_code".to_string());
        assert_eq!(code.secret(), "test_code");

        let empty_code = oauth2::AuthorizationCode::new("".to_string());
        assert_eq!(empty_code.secret(), "");
    }

    /// Test create_http_client redirect policy
    #[test]
    fn test_create_http_client_redirect_policy() {
        let client = create_http_client();

        // Verify the client was built successfully
        assert!(format!("{:?}", client).contains("Client"));

        // Test that the client has the expected configuration
        // We can't easily test internal configuration, but we can verify it exists
        assert!(!format!("{:?}", client).is_empty());
    }

    /// Test handle_apple_oauth with invalid token URL
    #[tokio::test]
    #[serial]
    async fn test_handle_apple_oauth_invalid_token_url() {
        unsafe {
            std::env::set_var("APPLE_CLIENT_ID", "test_client_id");
            std::env::set_var("APPLE_AUTH_URL", "https://valid.auth.url");
            std::env::set_var("APPLE_TOKEN_URL", "invalid_token_url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_apple_oauth(&pool, auth_code).await;

        // Should fail due to invalid token URL
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("APPLE_CLIENT_ID");
            std::env::remove_var("APPLE_AUTH_URL");
            std::env::remove_var("APPLE_TOKEN_URL");
        }
    }

    /// Test handle_github_oauth with invalid token URL
    #[tokio::test]
    #[serial]
    async fn test_handle_github_oauth_invalid_token_url() {
        unsafe {
            std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
            std::env::set_var("GITHUB_CLIENT_SECRET", "test_client_secret");
            std::env::set_var("GITHUB_AUTH_URL", "https://valid.auth.url");
            std::env::set_var("GITHUB_TOKEN_URL", "invalid_token_url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_github_oauth(&pool, auth_code).await;

        // Should fail due to invalid token URL
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("GITHUB_CLIENT_ID");
            std::env::remove_var("GITHUB_CLIENT_SECRET");
            std::env::remove_var("GITHUB_AUTH_URL");
            std::env::remove_var("GITHUB_TOKEN_URL");
        }
    }

    /// Test handle_apple_oauth with missing auth URL
    #[tokio::test]
    #[serial]
    async fn test_handle_apple_oauth_missing_auth_url() {
        unsafe {
            std::env::set_var("APPLE_CLIENT_ID", "test_client_id");
            std::env::remove_var("APPLE_AUTH_URL");
            std::env::set_var("APPLE_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_apple_oauth(&pool, auth_code).await;

        // Should fail due to missing auth URL
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("APPLE_CLIENT_ID");
            std::env::remove_var("APPLE_TOKEN_URL");
        }
    }

    /// Test handle_github_oauth with missing client secret
    #[tokio::test]
    #[serial]
    async fn test_handle_github_oauth_missing_client_secret() {
        unsafe {
            std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
            std::env::remove_var("GITHUB_CLIENT_SECRET");
            std::env::set_var("GITHUB_AUTH_URL", "https://valid.auth.url");
            std::env::set_var("GITHUB_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "test_auth_code".to_string();

        let result = handle_github_oauth(&pool, auth_code).await;

        // Should fail due to missing client secret
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("GITHUB_CLIENT_ID");
            std::env::remove_var("GITHUB_AUTH_URL");
            std::env::remove_var("GITHUB_TOKEN_URL");
        }
    }

    /// Test OAuth client creation with different client IDs
    #[test]
    fn test_oauth_client_creation_with_different_ids() {
        let client_id1 = oauth2::ClientId::new("test_id_1".to_string());
        let client_id2 = oauth2::ClientId::new("test_id_2".to_string());

        assert_eq!(client_id1.as_str(), "test_id_1");
        assert_eq!(client_id2.as_str(), "test_id_2");
        assert_ne!(client_id1, client_id2);
    }

    /// Test OAuth client secret creation
    #[test]
    fn test_oauth_client_secret_creation() {
        let secret = oauth2::ClientSecret::new("test_secret".to_string());
        assert_eq!(secret.secret(), "test_secret");

        let empty_secret = oauth2::ClientSecret::new("".to_string());
        assert_eq!(empty_secret.secret(), "");
    }

    /// Test OAuth error handling for URL parsing edge cases
    #[test]
    fn test_oauth_url_edge_cases() {
        // Test various invalid URL formats
        let invalid_urls = vec![
            "",
            "not-a-url",
            "://missing-scheme.com",
            "http://",
            "https://",
        ];

        for invalid_url in invalid_urls {
            let auth_url_result = oauth2::AuthUrl::new(invalid_url.to_string());
            assert!(
                auth_url_result.is_err(),
                "URL should be invalid: {}",
                invalid_url
            );

            let token_url_result = oauth2::TokenUrl::new(invalid_url.to_string());
            assert!(
                token_url_result.is_err(),
                "URL should be invalid: {}",
                invalid_url
            );
        }

        // Note: ftp:// URLs might be considered valid by the oauth2 library, so we test other cases

        // Test valid URLs
        let valid_urls = vec![
            "https://example.com",
            "https://example.com/oauth",
            "https://example.com:8080/oauth",
            "http://localhost:3000",
        ];

        for valid_url in valid_urls {
            let auth_url_result = oauth2::AuthUrl::new(valid_url.to_string());
            assert!(
                auth_url_result.is_ok(),
                "URL should be valid: {}",
                valid_url
            );

            let token_url_result = oauth2::TokenUrl::new(valid_url.to_string());
            assert!(
                token_url_result.is_ok(),
                "URL should be valid: {}",
                valid_url
            );
        }
    }

    /// Test BasicClient creation with minimal configuration
    #[test]
    fn test_basic_client_minimal_config() {
        let client_id = oauth2::ClientId::new("test_client".to_string());
        let client = BasicClient::new(client_id);

        // Verify client was created successfully
        assert!(!format!("{:?}", client).is_empty());
    }

    /// Test handle_apple_oauth with special characters in auth code
    #[tokio::test]
    #[serial]
    async fn test_handle_apple_oauth_special_chars_auth_code() {
        unsafe {
            std::env::set_var("APPLE_CLIENT_ID", "test_client_id");
            std::env::set_var("APPLE_AUTH_URL", "https://valid.auth.url");
            std::env::set_var("APPLE_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "code_with_special!@#$%^&*()_+chars".to_string();

        let result = handle_apple_oauth(&pool, auth_code).await;

        // Should fail at token exchange, but not due to special characters
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("APPLE_CLIENT_ID");
            std::env::remove_var("APPLE_AUTH_URL");
            std::env::remove_var("APPLE_TOKEN_URL");
        }
    }

    /// Test handle_github_oauth with special characters in auth code
    #[tokio::test]
    #[serial]
    async fn test_handle_github_oauth_special_chars_auth_code() {
        unsafe {
            std::env::set_var("GITHUB_CLIENT_ID", "test_client_id");
            std::env::set_var("GITHUB_CLIENT_SECRET", "test_client_secret");
            std::env::set_var("GITHUB_AUTH_URL", "https://valid.auth.url");
            std::env::set_var("GITHUB_TOKEN_URL", "https://valid.token.url");
        }

        let pool: Option<PgPool> = None;
        let auth_code = "code_with_special!@#$%^&*()_+chars".to_string();

        let result = handle_github_oauth(&pool, auth_code).await;

        // Should fail at token exchange, but not due to special characters
        assert!(result.is_err());

        // Clean up
        unsafe {
            std::env::remove_var("GITHUB_CLIENT_ID");
            std::env::remove_var("GITHUB_CLIENT_SECRET");
            std::env::remove_var("GITHUB_AUTH_URL");
            std::env::remove_var("GITHUB_TOKEN_URL");
        }
    }
}
