//! # Config Example
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

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};
use wasi_config::store as config;
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
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", get(config_get));
        wasi_http::serve(router, request).await
    }
}

/// Config request handler.
#[wasi_otel::instrument]
async fn config_get() -> Result<Json<Value>> {
    let config = config::get_all().expect("should get all");
    println!("config: {:?}", config);

    Ok(Json(json!({
        "message": "Hello, World!",
        "config": config
    })))
}
