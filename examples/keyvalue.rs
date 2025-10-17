#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use tracing::Level;
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};
use wasi_http::Result;
use wit_bindings::keyvalue::store;

struct HttpGuest;

impl Guest for HttpGuest {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let router = Router::new().route("/", post(handler));
        let out = wasi_http::serve(router, request);
        ResponseOutparam::set(response_out, out);
    }
}

#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Result<Json<Value>> {
    let bucket = store::open("credibil_bucket").context("opening bucket")?;
    bucket.set("my_key", &body).context("storing data")?;

    // check for previous value
    let res = bucket.get("my_key");
    tracing::debug!("found val: {res:?}");

    Ok(Json(json!({
        "message": "Hello, World!"
    })))
}

wasi::http::proxy::export!(HttpGuest);
