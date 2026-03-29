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
//! # CORS
//! Only `localhost` origins are permitted.
//!
//! # OpenAPI
//! Swagger UI is served at `GET /swagger-ui` (enabled by `utoipa-swagger-ui`).
//! The raw spec is at `GET /openapi.json`.

pub mod auth;
pub mod error;
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
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::AppState;

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
fn build_router(state: Arc<Mutex<AppState>>) -> Router {
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
        .allow_headers(Any);

    // Request body size limit: 200 MB (matches the import pipeline).
    let body_limit = RequestBodyLimitLayer::new(200 * 1024 * 1024);

    // Auth middleware applied to the API sub-router.
    let api_routes = Router::new()
        .route("/v1/health", get(routes::health))
        .route("/v1/verify", post(routes::verify_file))
        .route("/v1/verify/url", post(routes::verify_url))
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
        .route("/v1/auth/keys/:key_id", delete(routes::revoke_api_key))
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

/// Serve a minimal OpenAPI 3.1 specification.
async fn openapi_spec() -> impl axum::response::IntoResponse {
    let spec = serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Jura Trace Local API",
            "description": "Local REST API for the Jura Trace verification engine. Binds to 127.0.0.1:8300.",
            "version": env!("CARGO_PKG_VERSION"),
            "contact": {
                "name": "Juralabs CIC",
                "url": "https://juralabs.org"
            },
            "license": {
                "name": "PolyForm Noncommercial 1.0.0",
                "url": "https://polyformproject.org/licenses/noncommercial/1.0.0/"
            }
        },
        "servers": [{ "url": "http://127.0.0.1:8300" }],
        "security": [{ "bearerAuth": [] }],
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "jt_<key>",
                    "description": "API key with jt_ prefix. Obtain via POST /api/v1/auth/keys."
                }
            }
        },
        "paths": {
            "/api/v1/health": {
                "get": {
                    "summary": "Health check",
                    "description": "Returns server status and sidecar availability. No auth required.",
                    "security": [],
                    "operationId": "health",
                    "tags": ["System"],
                    "responses": {
                        "200": { "description": "Server is running" }
                    }
                }
            },
            "/api/v1/verify": {
                "post": {
                    "summary": "Verify a file",
                    "description": "Upload a file (image, video, audio, PDF) for full verification.",
                    "operationId": "verifyFile",
                    "tags": ["Verify"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "multipart/form-data": {
                                "schema": {
                                    "type": "object",
                                    "required": ["file"],
                                    "properties": {
                                        "file": { "type": "string", "format": "binary" },
                                        "mode": {
                                            "type": "string",
                                            "enum": ["quick", "standard", "deep", "archival"],
                                            "default": "standard"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Verification result" },
                        "400": { "description": "Invalid request" },
                        "401": { "description": "Unauthorized" }
                    }
                }
            },
            "/api/v1/verify/url": {
                "post": {
                    "summary": "Verify a URL",
                    "description": "Download and verify the content at a public URL.",
                    "operationId": "verifyUrl",
                    "tags": ["Verify"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["url"],
                                    "properties": {
                                        "url": { "type": "string", "format": "uri" },
                                        "mode": { "type": "string", "enum": ["quick", "standard", "deep", "archival"] }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Verification result" },
                        "400": { "description": "Invalid or unsafe URL" }
                    }
                }
            },
            "/api/v1/protect/sign": {
                "post": {
                    "summary": "Sign a file with C2PA credentials",
                    "operationId": "protectSign",
                    "tags": ["Protect"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "multipart/form-data": {
                                "schema": {
                                    "type": "object",
                                    "required": ["file", "creator_name"],
                                    "properties": {
                                        "file": { "type": "string", "format": "binary" },
                                        "creator_name": { "type": "string" },
                                        "license": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Signed file",
                            "content": { "application/octet-stream": {} }
                        }
                    }
                }
            },
            "/api/v1/protect/fingerprint": {
                "post": {
                    "summary": "Compute perceptual hashes",
                    "operationId": "protectFingerprint",
                    "tags": ["Protect"],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "multipart/form-data": {
                                "schema": {
                                    "type": "object",
                                    "required": ["file"],
                                    "properties": {
                                        "file": { "type": "string", "format": "binary" }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Hash values (aHash, dHash, pHash)" }
                    }
                }
            },
            "/api/v1/protect/watermark/embed": {
                "post": {
                    "summary": "Embed an invisible watermark",
                    "operationId": "protectWatermarkEmbed",
                    "tags": ["Protect"],
                    "responses": { "200": { "description": "Watermarked PNG image" } }
                }
            },
            "/api/v1/protect/watermark/extract": {
                "post": {
                    "summary": "Extract a watermark",
                    "operationId": "protectWatermarkExtract",
                    "tags": ["Protect"],
                    "responses": { "200": { "description": "Extracted watermark payload" } }
                }
            },
            "/api/v1/claims/check": {
                "post": {
                    "summary": "Verify a factual claim",
                    "operationId": "claimsCheck",
                    "tags": ["Claims"],
                    "responses": { "200": { "description": "Claim verification result" } }
                }
            },
            "/api/v1/stats": {
                "get": {
                    "summary": "Database statistics",
                    "operationId": "getStats",
                    "tags": ["System"],
                    "responses": { "200": { "description": "Stats summary" } }
                }
            },
            "/api/v1/auth/keys": {
                "post": {
                    "summary": "Create API key",
                    "operationId": "createApiKey",
                    "tags": ["Auth"],
                    "responses": { "200": { "description": "New key (shown once only)" } }
                },
                "get": {
                    "summary": "List API keys",
                    "operationId": "listApiKeys",
                    "tags": ["Auth"],
                    "responses": { "200": { "description": "Key list (no raw values)" } }
                }
            },
            "/api/v1/auth/keys/{key_id}": {
                "delete": {
                    "summary": "Revoke API key",
                    "operationId": "revokeApiKey",
                    "tags": ["Auth"],
                    "parameters": [{
                        "name": "key_id",
                        "in": "path",
                        "required": true,
                        "schema": { "type": "string" }
                    }],
                    "responses": { "200": { "description": "Key revoked" } }
                }
            }
        }
    });

    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        spec.to_string(),
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
