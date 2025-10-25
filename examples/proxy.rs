#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http_body_util::Empty;
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    // #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", get(handle_get)); //.route("/", post(handle_post));
        wasi_http::serve(router, request).await
    }
}

// Forward request to external service and return the response
// #[wasi_otel::instrument]
async fn handle_get() -> Result<Json<Value>> {
    let cache_expiry = "max-age=300";
    let cache_key = "qf55low9rjsrup46vsiz9r73";

    println!("Making GET request to external service");

    let request = http::Request::builder()
        .method(http::Method::GET)
        .uri("https://jsonplaceholder.cypress.io/posts/1")
        .header(CACHE_CONTROL, cache_expiry)
        .header(IF_NONE_MATCH, cache_key)
        .body(Empty::<Bytes>::new())
        .expect("Failed to build request");

    let _ = wasi_http::handle(request).await?;

    Ok(Json(json!({
        "response": "Hello, World! (from GET handler"
    })))
}

// // Forward request to external service and return the response
// #[wasi_otel::instrument]
// async fn handle_post(Json(body): Json<Value>) -> Result<Json<Value>> {
//     let body = Client::new()
//         .post("https://jsonplaceholder.cypress.io/posts")
//         .header(CACHE_CONTROL, "no-cache, max-age=300") // Go to origin for every request, but cache the response for 5 minutes
//         .bearer_auth("some token") // not required, but shown for example
//         .json(&body)
//         .send()?
//         .json::<Value>()
//         .context("issue sending request")?;

//     Ok(Json(json!({
//         "response": "Hello, World! (from POST handler",
//     })))
// }
