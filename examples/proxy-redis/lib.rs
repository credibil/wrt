#![cfg(target_arch = "wasm32")]

//! Minimal example of an HTTP proxy that uses Redis as a caching layer.
//!
//! See the handler functions below for the use of headers to control caching
//! behavior.

use std::convert::Infallible;

use anyhow::Context;
use axum::body::Body;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::{Empty, Full};
use serde_json::Value;
use tower_http::cors::{Any, CorsLayer};
use tracing::Level;
use wasi_http::{CacheOptions, Result};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct HttpGuest;
wasip3::http::proxy::export!(HttpGuest);

impl Guest for HttpGuest {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new()
            .route("/", get(get_handler))
            .layer(
                CorsLayer::new()
                    .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                    .allow_headers(Any)
                    .allow_origin(Any),
            )
            .route("/", post(post_handler));

        wasi_http::serve(router, request).await
    }
}

// Forward request to external service and return the response
#[wasi_otel::instrument]
async fn get_handler() -> Result<impl IntoResponse, Infallible> {
    let request = http::Request::builder()
        .method(Method::GET)
        .uri("https://jsonplaceholder.cypress.io/posts/1")
        .header(CACHE_CONTROL, "max-age=300") // enable caching for 5 minutes
        .header(IF_NONE_MATCH, "qf55low9rjsrup46vsiz9r73") // provide cache key
        .extension(CacheOptions {
            bucket_name: "example-bucket".to_string(),
        })
        .body(Empty::<Bytes>::new())
        .expect("failed to build request");
    let response = wasi_http::handle(request).await.unwrap();
    println!("response from wasi_http: {response:?}");
    let (parts, body) = response.into_parts();
    let http_response = http::Response::from_parts(parts, Body::from(body));
    println!("constructed http_response: {http_response:?}");
    Ok(http_response)
}

// Forward request to external service and return the response.
//
// This handler demonstrates caching a POST request response but will create an
// error: The Cache-Control header is set but the If-None-Match header is
// missing.
#[wasi_otel::instrument]
async fn post_handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    let body = serde_json::to_vec(&body).context("issue serializing request body")?;
    let body = Bytes::from(body);
    let request = http::Request::builder()
        .method(Method::POST)
        .uri("https://jsonplaceholder.cypress.io/posts")
        .header(CACHE_CONTROL, "no-cache, max-age=300") // Go to origin for every request, but cache the response for 5 minutes
        .body(Full::new(body))
        .expect("failed to build request");

    let response = wasi_http::handle(request).await?;
    let body = response.into_body();
    let body = serde_json::from_slice::<Value>(&body).context("issue parsing response body")?;

    Ok(Json(body))
}

// Handle preflight OPTIONS requests for CORS.
#[wasi_otel::instrument]
async fn handle_options() -> Result<()> {
    Ok(())
}
