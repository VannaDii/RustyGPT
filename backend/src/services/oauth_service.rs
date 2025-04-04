use oauth2::{AuthorizationCode, TokenResponse, basic::BasicClient};
use sqlx::PgPool;
use std::env;
use tracing::{error, info, instrument};
use uuid::Uuid;

#[instrument(skip(pool))]
pub async fn handle_apple_oauth(pool: &PgPool, auth_code: String) -> Result<Uuid, sqlx::Error> {
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

    let row = sqlx::query!("SELECT register_oauth_user(NULL, $1, NULL)", apple_id)
        .fetch_one(pool)
        .await?;
    info!(
        "Apple OAuth user registered with ID: {:?}",
        row.register_oauth_user
    );

    Ok(row.register_oauth_user.unwrap())
}

pub fn create_http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build")
}

#[instrument(skip(pool))]
pub async fn handle_github_oauth(pool: &PgPool, auth_code: String) -> Result<Uuid, sqlx::Error> {
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

    let row = sqlx::query!("SELECT register_oauth_user(NULL, NULL, $1)", github_id)
        .fetch_one(pool)
        .await?;
    info!(
        "GitHub OAuth user registered with ID: {:?}",
        row.register_oauth_user
    );

    Ok(row.register_oauth_user.unwrap())
}
