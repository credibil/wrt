//! # HTTP Server Wasm Guest
//!
//! This module demonstrates the basic WASI HTTP handler pattern. It shows how to:
//! - Implement the WASI HTTP `Guest` trait
//! - Use Axum for routing within a WebAssembly guest
//! - Handle JSON request/response bodies
//! - Integrate OpenTelemetry tracing
//!
//! ## How It Works
//!
//! 1. The host receives an HTTP request on the network
//! 2. The host calls the guest's `handle()` function with the request
//! 3. The guest routes the request using Axum and processes it
//! 4. The response is returned to the host, which sends it to the client

#![cfg(target_arch = "wasm32")]

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct HttpGuest;

// This macro generates the WebAssembly exports required by the WASI HTTP
// interface. It creates the necessary FFI glue code so the host can invoke
// our `Guest` implementation.
wasip3::http::proxy::export!(HttpGuest);

impl Guest for HttpGuest {
    /// Main HTTP request handler.
    ///
    /// This function is called by the host for every incoming HTTP request.
    /// We use Axum's `Router` to dispatch requests to the appropriate handler
    /// based on the HTTP method and path.
    ///
    /// The `#[wasi_otel::instrument]` attribute automatically creates an
    /// OpenTelemetry span for this function, enabling distributed tracing.
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        // Build the Axum router with our endpoints.
        // Both GET and POST on "/" are handled, demonstrating method-based routing.
        let router = Router::new().route("/", get(echo_get)).route("/", post(echo_post));

        // `wasi_http::serve` bridges Axum's request/response types with WASI HTTP types.
        // This is the key integration point that makes Axum work in WebAssembly.
        wasi_http::serve(router, request).await
    }
}

/// Handler for GET requests.
///
/// Returns a JSON response with a greeting message. The `Json` extractor
/// automatically parses the request body as JSON (if present).
#[wasi_otel::instrument]
async fn echo_get(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}

/// Handler for POST requests.
///
/// Accepts a JSON body and echoes it back in the response. This demonstrates
/// how to receive and process JSON payloads in your handlers.
#[wasi_otel::instrument]
async fn echo_post(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}
