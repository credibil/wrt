//! Minimal example of an HTTP proxy that uses Redis as a caching layer.
//!
//! See the handler functions below for the use of headers to control caching
//! behavior.

#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::{get, post};
use axum::{Json, Router};
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use sdk_http::{Client, Decode, Result};
use serde_json::{Value, json};
use tower_http::cors::{Any, CorsLayer};
use tracing::Level;
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};

struct HttpGuest;

impl Guest for HttpGuest {
    #[sdk_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let router = Router::new()
            .route("/", get(get_handler))
            .layer(
                CorsLayer::new()
                    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                    .allow_headers(Any)
                    .allow_origin(Any),
            )
            .route("/", post(post_handler));

        let out = sdk_http::serve(router, request);
        ResponseOutparam::set(response_out, out);
    }
}

// Forward request to external service and return the response
#[sdk_otel::instrument]
async fn get_handler() -> Result<Json<Value>> {
    let body = Client::new()
        .cache_bucket("credibil_bucket")
        .get("https://jsonplaceholder.cypress.io/posts/1")
        .header(CACHE_CONTROL, "max-age=300") // enable caching for 5 minutes
        .header(IF_NONE_MATCH, "qf55low9rjsrup46vsiz9r73") // provide cache key
        .send()?
        .json::<Value>()
        .context("issue sending request")?;

    Ok(Json(json!({
        "response": body
    })))
}

// Forward request to external service and return the response.
//
// This handler demonstrates caching a POST request response but will create an
// error: The Cache-Control header is set but the If-None-Match header is
// missing.
#[sdk_otel::instrument]
async fn post_handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    let body = Client::new()
        .cache_bucket("credibil_bucket")
        .post("https://jsonplaceholder.cypress.io/posts")
        .header(CACHE_CONTROL, "no-cache, max-age=300") // Go to origin for every request, but cache the response for 5 minutes
        .bearer_auth("some token") // not required, but shown for example
        .json(&body)
        .send()?
        .json::<Value>()
        .context("issue sending request")?;

    Ok(Json(json!({
        "response": body
    })))
}

// Handle preflight OPTIONS requests for CORS.
#[sdk_otel::instrument]
async fn handle_options() -> Result<()> {
    Ok(())
}

wasi::http::proxy::export!(HttpGuest);
