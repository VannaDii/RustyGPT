use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "RustyGPT API",
        version = "1.0.0",
        description = "API documentation for RustyGPT"
    ),
    paths(), // ...existing endpoints...
    components(
        schemas() // ...existing schemas...
    )
)]
pub struct ApiDoc;
