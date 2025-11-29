#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasi_identity::credentials::get_identity;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};
use wit_bindgen::block_on;

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::INFO)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", get(handler));
        wasi_http::serve(router, request).await
    }
}

#[wasi_otel::instrument]
async fn handler() -> Result<Json<Value>> {
    let identity = block_on(get_identity("identity".to_string())).context("gettting identity")?;
    let access_token = block_on(async move { identity.get_token(vec![]).await })
        .context("getting access token")?;
    
    println!("{}", access_token.token);

    Ok(Json(json!({
        "message": "Hello, World!"
    })))
}
