// Application state that will be shared across all routes
#[derive(Clone, Default)]
pub struct AppState {
    pub(crate) pool: Option<sqlx::PgPool>,
}
