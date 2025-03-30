// Application state that will be shared across all routes
#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: sqlx::PgPool,
}
