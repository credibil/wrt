#![cfg(target_arch = "wasm32")]

use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    // #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    // async fn handle(request: Request) -> Result<Response, ErrorCode> {
    //     tracing::info!("received request");
    //     let _ = handler2();
    //     let router = Router::new().route("/", post(handler));
    //     wasi_http::serve(router, request).await
    // }

    #[allow(clippy::future_not_send)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let _guard = ::wasi_otel::init();
        
        ::tracing::Instrument::instrument(
            async move {
                tracing::info!("received request");
                let _ = handler2();
                let router = Router::new().route("/", post(handler));
                wasi_http::serve(router, request).await
            },
            ::tracing::span!(Level::INFO, "http_guest_handle"),
        )
        .await
    }
}

fn handler2() -> Result<()> {
    ::tracing::span!(::tracing::Level::DEBUG, "handler2").in_scope(|| Ok(()))
}

// #[wasi_otel::instrument]
// fn handler2() -> Result<()> {
//     Ok(())
// }

#[axum::debug_handler]
// #[wasi_otel::instrument]
async fn handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    println!("handler span active: {:?}", ::tracing::Span::current());
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}
