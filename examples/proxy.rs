#![cfg(target_arch = "wasm32")]
use anyhow::Context;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64ct::{Base64, Encoding};
use bytes::Bytes;
use http::Method;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::{Empty, Full};
use serde_json::Value;
use tracing::Level;
use wasi_http::{CacheOptions, Result};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router =
            Router::new().route("/cache", get(handle_cache)).route("/origin", post(handle_origin));
        wasi_http::serve(router, request).await
    }
}

// Attempt to get cached response with fallback to origin
#[wasi_otel::instrument]
async fn handle_cache() -> Result<Json<Value>> {
    let max_age = "max-age=300";
    let cache_key = "qf34lofge4rstep95tsiz9r75";
    // PEM-encoded private key and certificate
    let auth_cert = "-----BEGIN CERTIFICATE-----
...Your Certificate Here...
-----END CERTIFICATE----- 
-----BEGIN PRIVATE KEY-----
...Your Private Key Here...
-----END PRIVATE KEY-----";
    let encoded_cert = Base64::encode_string(auth_cert.as_bytes());
    let request = http::Request::builder()
        .method(Method::GET)
        .uri("https://jsonplaceholder.cypress.io/posts/1")
        .header(CACHE_CONTROL, max_age)
        .header(IF_NONE_MATCH, cache_key)
        .header("Client-Cert", &encoded_cert)
        .extension(CacheOptions {
            bucket_name: "example-bucket".to_string(),
            ttl_seconds: 300,
        })
        .body(Empty::<Bytes>::new())
        .expect("Failed to build request");
    let response = wasi_http::handle(request).await?;
    let body = response.into_body();
    // let body = serde_json::from_slice::<Value>(&body).context("issue parsing response body")?;

    let body_str = Base64::encode_string(&body);

    Ok(Json(serde_json::json!({
        "cached_response": body_str
    })))
}

// Forward request to external service and cache the response
#[wasi_otel::instrument]
async fn handle_origin(body: Bytes) -> Result<Json<Value>> {
    let max_age = "max-age=300";
    let cache_key = "qf34lofge4rstep95tsiz9r75";

    let request = http::Request::builder()
        .method(Method::GET)
        .uri("https://jsonplaceholder.cypress.io/posts")
        .header(CACHE_CONTROL, max_age)
        .header(IF_NONE_MATCH, cache_key)
        .body(Full::new(body))
        .expect("Failed to build request");

    let response = wasi_http::handle(request).await?;
    let body = response.into_body();
    let body = serde_json::from_slice::<Value>(&body).context("issue parsing response body")?;

    Ok(Json(body))
}
