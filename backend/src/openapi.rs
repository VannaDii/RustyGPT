use shared::models::{
    Conversation, ErrorResponse, SetupRequest, SetupResponse, conversation::SendMessageResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "RustyGPT API",
        version = "1.0.0",
        description = "API documentation for RustyGPT"
    ),
    paths(
        crate::handlers::setup::get_setup,
        crate::handlers::setup::post_setup,
        crate::handlers::github_auth::github_oauth_init,
        crate::handlers::github_auth::github_oauth_callback,
        crate::handlers::github_auth::github_oauth_manual,
        crate::handlers::apple_auth::apple_oauth_init,
        crate::handlers::apple_auth::apple_oauth_callback,
        crate::handlers::apple_auth::apple_oauth_manual,
        crate::handlers::conversation::get_conversation,
        crate::handlers::conversation::send_message,
    ),
    components(
        schemas(
            SetupRequest,
            SetupResponse,
            ErrorResponse,
            Conversation,
            SendMessageResponse,
        )
    ),
    tags(
        (name = "Setup", description = "Setup-related endpoints"),
        (name = "Auth", description = "Authentication-related endpoints"),
        (name = "Chat", description = "Chat-related endpoints")
    )
)]
pub struct ApiDoc;
