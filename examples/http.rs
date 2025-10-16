#![cfg(target_arch = "wasm32")]

use axum::routing::post;
use axum::{Json, Router};
use sdk_http::Result;
use serde_json::{Value, json};
use tracing::Level;
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};

struct HttpGuest;
wasi::http::proxy::export!(HttpGuest);

impl Guest for HttpGuest {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        tracing::info!("received request");
        let router = Router::new().route("/", post(handler));
        let out = sdk_http::serve(router, request);
        ResponseOutparam::set(response_out, out);
    }
}

// A simple "Hello, World!" endpoint that returns the client's request.
#[axum::debug_handler]
#[wasi_otel::instrument]
async fn handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}
