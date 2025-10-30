#![cfg(target_arch = "wasm32")]
#![allow(missing_docs)]

use std::println;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use wasi_http::Result;
use wasi_websockets::handler;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct HttpGuest;
wasip3::http::proxy::export!(HttpGuest);

impl Guest for HttpGuest {
    //#[wasi_otel::instrument(name = "http_guest_handle")]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router =
            Router::new().route("/health", get(get_handler)).route("/socket", post(post_handler));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
//#[wasi_otel::instrument]
async fn get_handler() -> Result<Json<Value>> {
    let status = handler::health_check();

    Ok(Json(json!({
        "message": status.unwrap()
    })))
}

/// Example POST handler that sends a message to websocket peers
#[axum::debug_handler]
//#[wasi_otel::instrument]
async fn post_handler(body: String) -> Result<Json<Value>> {
    let peers = handler::get_peers();
    let client_peers = peers.unwrap();
    let recipients: Vec<String> = client_peers
        .iter()
        .filter(|p| !(p.address.starts_with("127.0.0.1") || p.address.starts_with("localhost")))
        .map(|p| p.address.clone())
        .collect();
    let result = handler::send_peers(&body, &recipients);
    if let Err(e) = result {
        println!("Error sending websocket message: {e}");
    }

    Ok(Json(json!({
        "message": "message received"
    })))
}
