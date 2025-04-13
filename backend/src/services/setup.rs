use shared::models::SetupRequest;
use sqlx::{Error, PgPool};

/// Checks if the database is already set up.
pub async fn is_setup(pool: &Option<PgPool>) -> Result<bool, Error> {
    let row = sqlx::query!("SELECT is_setup() AS configured")
        .fetch_one(pool.as_ref().unwrap())
        .await?;

    Ok(row.configured.unwrap_or(false))
}

/// Performs the setup.
pub async fn init_setup(pool: &Option<PgPool>, config: &SetupRequest) -> Result<bool, Error> {
    // Check if the database is already set up
    if is_setup(pool).await? {
        return Ok(false);
    }

    // Perform the setup
    let result = sqlx::query!(
        "SELECT init_setup($1, $2, $3) AS success",
        config.username,
        config.email,
        config.password,
    )
    .fetch_one(pool.as_ref().unwrap())
    .await?;

    Ok(result.success.unwrap_or(false))
}
