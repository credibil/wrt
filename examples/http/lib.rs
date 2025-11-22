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
            .route("/echo", get(echo))
            .route("/cache", get(cache))
            .route("/origin", post(origin))
            .route("/client-cert", post(client_cert));
        wasi_http::serve(router, request).await
    }
}

// Simple handler that echoes back the request body with a greeting
#[wasi_otel::instrument]
async fn echo(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}

// Forward request to external service and return the response
#[wasi_otel::instrument]
async fn cache() -> Result<impl IntoResponse, Infallible> {
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
    let (parts, body) = response.into_parts();
    let http_response = http::Response::from_parts(parts, Body::from(body));

    Ok(http_response)
}

// Forward request to external service and return the response.
//
// While the proxy will cache the response, an error will be returned as the
// `If-None-Match` header is missing.
#[wasi_otel::instrument]
async fn origin(body: Bytes) -> Result<Json<Value>> {
    let request = http::Request::builder()
        .method(Method::POST)
        .uri("https://jsonplaceholder.cypress.io/posts")
        // go to origin for every request, but cache response for 5 minutes
        .header(CACHE_CONTROL, "no-cache, max-age=300") 
        .body(Full::new(body))
        .expect("failed to build request");

    let response = wasi_http::handle(request).await?;
    let body = response.into_body();
    let body = serde_json::from_slice::<Value>(&body).context("issue parsing response body")?;

    Ok(Json(body))
}

#[wasi_otel::instrument]
async fn client_cert() -> Result<Json<Value>> {
    // PEM-encoded private key and certificate
    let auth_cert = "
        -----BEGIN CERTIFICATE-----
        ...Your Certificate Here...
        -----END CERTIFICATE----- 
        -----BEGIN PRIVATE KEY-----
        ...Your Private Key Here...
        -----END PRIVATE KEY-----";
    let encoded_cert = Base64::encode_string(auth_cert.as_bytes());

    let request = http::Request::builder()
        .method(Method::GET)
        .uri("https://jsonplaceholder.cypress.io/posts/1")
        .header("Client-Cert", &encoded_cert)
        .extension(CacheOptions {
            bucket_name: "example-bucket".to_string(),
        })
        .body(Empty::<Bytes>::new())
        .expect("Failed to build request");

    let response = wasi_http::handle(request).await?;
    let body = response.into_body();
    let body_str = Base64::encode_string(&body);

    Ok(Json(serde_json::json!({
        "cached_response": body_str
    })))
}
