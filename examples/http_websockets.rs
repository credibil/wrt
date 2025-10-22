#![cfg(target_arch = "wasm32")]
#![allow(missing_docs)]

use std::println;

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};
use wasi_http::Result;
use wasi_websockets::handler;

struct HttpGuest;
wasi::http::proxy::export!(HttpGuest);

impl Guest for HttpGuest {
    //#[wasi_otel::instrument(name = "http_guest_handle")]
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let router =
            Router::new().route("/health", get(get_handler)).route("/socket", post(post_handler));
        let out = wasi_http::serve(router, request);
        ResponseOutparam::set(response_out, out);
    }
}

#[axum::debug_handler]
//#[wasi_otel::instrument]
async fn get_handler() -> Result<()> {
    println!("handling health probe");

    Ok(())
}

#[axum::debug_handler]
//#[wasi_otel::instrument]
async fn post_handler(body: String) -> Result<Json<Value>> {
    println!("handling websocket connection");
    let result = handler::send(&body);
    if let Err(e) = result {
        println!("Error sending websocket message: {}", e);
    }

    Ok(Json(json!({
        "message": "message received"
    })))
}
