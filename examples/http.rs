#![cfg(all(target_arch = "wasm32", feature = "http"))]

use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        tracing::info!("received request");
        let _ = handler2();
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

#[wasi_otel::instrument]
fn handler2() -> Result<()> {
    Ok(())
}

#[axum::debug_handler]
#[wasi_otel::instrument]
async fn handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}
