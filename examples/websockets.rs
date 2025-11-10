#![cfg(target_arch = "wasm32")]

use std::println;

use anyhow::{Context, anyhow};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use wasi_http::Result;
use wasi_websockets::store;
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
    let server = store::get_server().context("getting websocket server")?;
    let message = server.health_check().unwrap_or_else(|_| "Service is unhealthy".to_string());

    Ok(Json(json!({
        "message": message
    })))
}

/// Example POST handler that sends a message to websocket peers
#[axum::debug_handler]
//#[wasi_otel::instrument]
async fn post_handler(body: String) -> Result<Json<Value>> {
    let server = store::get_server().context("getting websocket server")?;
    let peers = server.get_peers();
    let client_peers = match peers {
        Ok(p) => p,
        Err(e) => {
            println!("Error retrieving websocket peers: {e}");
            return Err(anyhow!("error retrieving websocket peers").into());
        }
    };
    let recipients: Vec<String> = client_peers.iter().map(|p| p.address.clone()).collect();

    if let Err(e) = server.send_peers(&body, &recipients) {
        println!("Error sending websocket message: {e}");
    }

    Ok(Json(json!({
        "message": "message received"
    })))
}
