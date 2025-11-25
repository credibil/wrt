#![cfg(target_arch = "wasm32")]

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
// use tracing::Level;
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct HttpGuest;
wasip3::http::proxy::export!(HttpGuest);

impl Guest for HttpGuest {
    // #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", get(echo_get)).route("/", post(echo_post));
        wasi_http::serve(router, request).await
    }
}

// #[wasi_otel::instrument]
async fn echo_get(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}

// #[wasi_otel::instrument]
async fn echo_post(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}
