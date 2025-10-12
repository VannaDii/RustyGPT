use std::sync::Arc;

use axum::{
    Extension, Router,
    extract::Path,
    http::{HeaderValue, StatusCode, header},
    response::Response,
    routing::get,
};

use crate::app_state::AppState;
use shared::config::server::{Config, WellKnownEntry};

fn build_response(entry: &WellKnownEntry) -> Response {
    let mut response = Response::new(entry.body.clone().into());
    let content_type = HeaderValue::from_str(&entry.content_type)
        .unwrap_or_else(|_| HeaderValue::from_static("text/plain"));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("max-age=3600, public"),
    );
    response
}

async fn well_known_handler(
    Extension(config): Extension<Arc<Config>>,
    Path(path): Path<String>,
) -> Result<Response, StatusCode> {
    if !config.features.well_known {
        return Err(StatusCode::NOT_FOUND);
    }

    let requested_path = format!(".well-known/{path}");
    if let Some(entry) = config
        .well_known
        .entries
        .iter()
        .find(|candidate| candidate.path == requested_path)
    {
        Ok(build_response(entry))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

pub fn create_router_well_known() -> Router<Arc<AppState>> {
    Router::new().route("/.well-known/:path", get(well_known_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Extension;
    use shared::config::server::{Config, Profile, WellKnownEntry};

    fn config_with_entry() -> Arc<Config> {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.features.well_known = true;
        config.well_known.entries = vec![WellKnownEntry {
            path: ".well-known/security.txt".into(),
            content_type: "text/plain".into(),
            body: "Contact: mailto:security@example.com".into(),
        }];
        Arc::new(config)
    }

    #[tokio::test]
    async fn returns_well_known_entry_when_enabled() {
        let config = config_with_entry();

        let response =
            super::well_known_handler(Extension(config), Path("security.txt".to_string()))
                .await
                .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn returns_not_found_when_disabled() {
        let mut config = Config::default_for_profile(Profile::Dev);
        config.features.well_known = false;
        config.well_known.entries = vec![WellKnownEntry {
            path: ".well-known/security.txt".into(),
            content_type: "text/plain".into(),
            body: "Contact: mailto:security@example.com".into(),
        }];
        let config = Arc::new(config);

        let response =
            super::well_known_handler(Extension(config), Path("security.txt".to_string())).await;

        assert_eq!(response.unwrap_err(), StatusCode::NOT_FOUND);
    }
}
