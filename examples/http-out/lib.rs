#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::{get, post};
use axum::{Json, Router};
use http::Method;
use wasi_http::{Client, Decode, Result};
use serde_json::{Value, json};
use tower_http::cors::{Any, CorsLayer};
use tracing::Level;
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};

struct HttpGuest;

impl Guest for HttpGuest {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
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

        let out = wasi_http::serve(router, request);
        ResponseOutparam::set(response_out, out);
    }
}

// Forward request to external service and return the response
#[wasi_otel::instrument]
async fn get_handler() -> Result<Json<Value>> {
    let body = Client::new()
        .get("https://jsonplaceholder.cypress.io/posts/1")
        .send()?
        .json::<Value>()
        .context("issue sending request")?;

    Ok(Json(json!({
        "response": body
    })))
}

// Forward request to external service and return the response
#[wasi_otel::instrument]
async fn post_handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    let body = Client::new()
        .post("https://jsonplaceholder.cypress.io/posts")
        .bearer_auth("some token") // not required, but shown for example
        .json(&body)
        .send()?
        .json::<Value>()
        .context("issue sending request")?;

    Ok(Json(json!({
        "response": body
    })))
}

wasi::http::proxy::export!(HttpGuest);
