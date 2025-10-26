#![cfg(target_arch = "wasm32")]

use anyhow::{Context, anyhow};
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::{Empty, Full};
use serde_json::Value;
use tracing::Level;
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    // #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", get(handle_cache)).route("/", post(handle_origin));
        wasi_http::serve(router, request).await
    }
}

// Attempt to get cached response with fallback to origin
// #[wasi_otel::instrument]
async fn handle_cache() -> Result<Json<Value>> {
    let max_age = "max-age=300";
    let cache_key = "qf55low9rjsrup46vsiz9r73";

    // let request = http::Request::builder()
    //     .method(Method::GET)
    //     .uri("https://jsonplaceholder.cypress.io/posts/1")
    //     .header(CACHE_CONTROL, max_age)
    //     .header(IF_NONE_MATCH, cache_key)
    //     .body(Empty::<Bytes>::new())
    //     .expect("Failed to build request");

    // let response = wasi_http::handle(request).await?;
    // let body = response.into_body();
    // let body = serde_json::from_slice::<Value>(&body).context("issue parsing response body")?;

    let body = reqwest_p3::Client::new()
        .get("https://jsonplaceholder.cypress.io/posts/1")
        .send()
        .await
        .map_err(|e| anyhow!("{e}"))?
        .json::<Value>()
        .map_err(|e| anyhow!("{e}"))?;

    Ok(Json(body))
}

// Forward request to external service and cache the response
#[wasi_otel::instrument]
async fn handle_origin(body: Bytes) -> Result<Json<Value>> {
    let max_age = "max-age=300";

    let request = http::Request::builder()
        .method(Method::GET)
        .uri("https://jsonplaceholder.cypress.io/posts")
        .header(CACHE_CONTROL, max_age)
        .body(Full::new(body))
        .expect("Failed to build request");

    let response = wasi_http::handle(request).await?;
    let body = response.into_body();
    let body = serde_json::from_slice::<Value>(&body).context("issue parsing response body")?;

    Ok(Json(body))
}
