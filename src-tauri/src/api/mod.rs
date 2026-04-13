//! Local REST API wrapper for Jura Trace (Phase A, PV-A2).
//!
//! Exposes the core Rust verification engine as an HTTP API on port 8300.
//! The server binds exclusively to `127.0.0.1` — it is never reachable from
//! other machines.
//!
//! # Authentication
//! Every endpoint except `GET /api/v1/health` requires a valid API key
//! supplied via `Authorization: Bearer jt_<key>`.  Keys are created via
//! `POST /api/v1/auth/keys` or auto-generated on first launch.
//!
//! # Rate Limiting
//! Each key is limited to `rate_limit` requests per minute (default 100).
//! Exceeded requests receive HTTP 429 with `X-RateLimit-*` headers.
//!
//! # CORS
//! Only `localhost` origins are permitted.
//!
//! # OpenAPI
//! The spec is served at `GET /openapi.json` (utoipa-generated).
//! Swagger UI is served at `GET /swagger-ui/`.

pub mod auth;
pub mod error;
pub mod rate_limit;
pub mod routes;
pub mod types;

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::{
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;

use crate::AppState;
use rate_limit::RateLimiter;

// ── OpenAPI document ─────────────────────────────────────────────────────────

/// Root OpenAPI document assembled from utoipa derive macros.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Jura Trace Local API",
        description = "Local REST API for the Jura Trace verification engine. \
                       Binds to 127.0.0.1:8300. All endpoints except /api/v1/health \
                       require Bearer authentication.",
        version = env!("CARGO_PKG_VERSION"),
        contact(name = "Juralabs CIC", url = "https://juralabs.org"),
        license(
            name = "PolyForm Noncommercial 1.0.0",
            url = "https://polyformproject.org/licenses/noncommercial/1.0.0/"
        )
    ),
    servers(
        (url = "http://127.0.0.1:8300", description = "Local API server")
    ),
    security(
        ("bearerAuth" = [])
    ),
    components(
        schemas(
            types::HealthResponse,
            types::VerifyUrlRequest,
            types::ClaimCheckRequest,
            types::CreateKeyRequest,
            types::CreateKeyResponse,
            types::StatsResponse,
            types::FingerprintEntry,
            types::FingerprintResponse,
            types::BatchVerifyItem,
            types::BatchVerifyResponse,
        )
    ),
    modifiers(&BearerSecurityAddon),
    paths(
        routes::health,
        routes::verify_file,
        routes::verify_url,
        routes::protect_sign,
        routes::protect_fingerprint,
        routes::protect_watermark_embed,
        routes::protect_watermark_extract,
        routes::claims_check,
        routes::get_stats,
        routes::create_api_key,
        routes::list_api_keys_handler,
        routes::revoke_api_key,
        routes::verify_batch,
    ),
    tags(
        (name = "System", description = "Server health and statistics"),
        (name = "Verify", description = "Content verification endpoints"),
        (name = "Protect", description = "Content protection and signing"),
        (name = "Claims", description = "Factual claim verification"),
        (name = "Auth", description = "API key management"),
    )
)]
pub struct ApiDoc;

/// Modifier that injects the `bearerAuth` security scheme into the OpenAPI
/// `components/securitySchemes` map. Using a modifier avoids the deprecated
/// `security_schemes` attribute syntax in utoipa v5.
struct BearerSecurityAddon;

impl utoipa::Modify for BearerSecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("jt_<key>")
                    .description(Some(
                        "API key with jt_ prefix. Obtain via POST /api/v1/auth/keys.",
                    ))
                    .build(),
            ),
        );
    }
}

// ── Server entry point ───────────────────────────────────────────────────────

/// Start the Axum HTTP server on `127.0.0.1:{port}`.
///
/// If the port is already in use (e.g. another Jura Trace instance), a warning
/// is logged and the function returns normally — the Tauri app continues without
/// the API server.
pub async fn start_server(
    state: Arc<Mutex<AppState>>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    routes::init_start_time();

    // Build the router.
    let app = build_router(state);

    // Bind to loopback only — never expose to external interfaces.
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    log::info!("API server listening on http://{addr}");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            log::warn!(
                "API server: could not bind to {addr}: {e}. \
                 The REST API will be unavailable for this session."
            );
            return Ok(());
        }
    };

    axum::serve(listener, app).await?;

    Ok(())
}

/// Assemble the full Axum router with middleware.
pub fn build_router(state: Arc<Mutex<AppState>>) -> Router {
    // CORS: allow only localhost origins.
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:1420".parse().unwrap(),
            "http://127.0.0.1:1420".parse().unwrap(),
            "http://localhost:8300".parse().unwrap(),
            "http://127.0.0.1:8300".parse().unwrap(),
        ])
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    // Request body size limit: 200 MB (matches the import pipeline).
    let body_limit = RequestBodyLimitLayer::new(200 * 1024 * 1024);

    // Shared rate limiter — one instance per server lifetime.
    let limiter = RateLimiter::new();

    // Auth middleware applied to the API sub-router.
    // Rate limiter runs after auth (needs AuthenticatedKey extension).
    let api_routes = Router::new()
        .route("/v1/health", get(routes::health))
        .route("/v1/verify", post(routes::verify_file))
        .route("/v1/verify/url", post(routes::verify_url))
        .route("/v1/verify/batch", post(routes::verify_batch))
        .route("/v1/protect/sign", post(routes::protect_sign))
        .route("/v1/protect/fingerprint", post(routes::protect_fingerprint))
        .route(
            "/v1/protect/watermark/embed",
            post(routes::protect_watermark_embed),
        )
        .route(
            "/v1/protect/watermark/extract",
            post(routes::protect_watermark_extract),
        )
        .route("/v1/claims/check", post(routes::claims_check))
        .route("/v1/stats", get(routes::get_stats))
        .route("/v1/auth/keys", post(routes::create_api_key))
        .route("/v1/auth/keys", get(routes::list_api_keys_handler))
        .route("/v1/auth/keys/{key_id}", delete(routes::revoke_api_key))
        .layer(middleware::from_fn_with_state(
            (state.clone(), limiter),
            rate_limit::rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ))
        .with_state(state.clone());

    // OpenAPI spec and Swagger UI (no auth).
    let openapi_routes = Router::new()
        .route("/openapi.json", get(openapi_spec))
        .route("/swagger-ui", get(swagger_ui_redirect))
        .route("/swagger-ui/", get(swagger_ui_html));

    Router::new()
        .nest("/api", api_routes)
        .merge(openapi_routes)
        .layer(cors)
        .layer(body_limit)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// ── OpenAPI spec endpoint ────────────────────────────────────────────────────

/// `GET /openapi.json` — return the utoipa-generated OpenAPI 3.1 specification.
async fn openapi_spec() -> impl axum::response::IntoResponse {
    let spec = ApiDoc::openapi()
        .to_json()
        .unwrap_or_else(|_| "{}".to_string());

    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        spec,
    )
}

/// Redirect `/swagger-ui` → `/swagger-ui/`.
async fn swagger_ui_redirect() -> impl axum::response::IntoResponse {
    axum::response::Redirect::permanent("/swagger-ui/")
}

/// Serve a simple Swagger UI HTML page pointing at the local OpenAPI spec.
async fn swagger_ui_html() -> impl axum::response::IntoResponse {
    let html = r#"<!DOCTYPE html>
<html>
<head>
  <title>Jura Trace API</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" >
</head>
<body>
<div id="swagger-ui"></div>
<script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"> </script>
<script>
  window.onload = function() {
    const ui = SwaggerUIBundle({
      url: "/openapi.json",
      dom_id: '#swagger-ui',
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: "BaseLayout"
    })
    window.ui = ui
  }
</script>
</body>
</html>"#;

    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}
