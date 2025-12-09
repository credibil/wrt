#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::Value;
use tracing::Level;
use wasi_http::Result;
use wasi_vault::vault;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Result<Json<Value>> {
    // write secret to vault
    let locker = vault::open("credibil-locker").context("failed to open vault locker")?;
    locker.set("secret-id", &body).context("issue setting secret")?;

    // read secret from vault
    let secret = locker.get("secret-id").context("issue retriving secret")?;
    assert_eq!(secret.unwrap(), body);

    let response = serde_json::from_slice::<Value>(&body).context("deserializing data")?;
    tracing::debug!("sending response: {response:?}");
    Ok(Json(response))
}
