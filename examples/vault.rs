#![cfg(target_arch = "wasm32")]

use anyhow::{Context, anyhow};
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::Value;
use tracing::Level;
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};
use wasi_http::Result;
use wasi_vault::vault;

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
    // write secret to vault
    let locker =
        vault::open("credibil-locker").map_err(|e| anyhow!("failed to open vault locker: {e}"))?;
    locker.set("secret-id", &body).map_err(|e| anyhow!("issue setting secret: {e}"))?;

    // read secret from vault
    let secret = locker.get("secret-id").map_err(|e| anyhow!("issue retriving secret: {e}"))?;
    assert_eq!(secret.unwrap(), body);

    let response = serde_json::from_slice::<Value>(&body).context("deserializing data")?;
    tracing::debug!("sending response: {response:?}");
    Ok(Json(response))
}

wasi::http::proxy::export!(HttpGuest);
