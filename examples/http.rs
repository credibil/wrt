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
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        tracing::info!("received request");
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }

    // #[allow(clippy::future_not_send)]
    // async fn handle(request: Request) -> Result<Response, ErrorCode> {
    //     let _guard = ::tracing::Span::current().is_none().then(::wasi_otel::init);
    //     ::tracing::Instrument::instrument(
    //         {
    //             tracing::info!("received request");
    //             let router = Router::new().route("/", post(handler));
    //             wasi_http::serve(router, request).await
    //         },
    //         ::tracing::span!(Level::DEBUG, "http_guest_handle"),
    //     )
    //     .into_inner()
    // }

    // #[allow(refining_impl_trait, reason = "`Future` return type needs to be marked as `Send`")]
    // fn handle(request: Request) -> impl Future<Output = Result<Response, ErrorCode>> + Send {
    //     let _guard = ::tracing::Span::current().is_none().then(::wasi_otel::init);
    //     async move {
    //         ::tracing::Instrument::instrument(
    //             {
    //                 tracing::info!("received request");
    //                 let router = Router::new().route("/", post(handler));
    //                 wasi_http::serve(router, request).await
    //             },
    //             ::tracing::span!(Level::DEBUG, "http_guest_handle"),
    //         )
    //         .into_inner()
    //     }
    // }
}

#[axum::debug_handler]
#[wasi_otel::instrument]
async fn handler(Json(body): Json<Value>) -> Result<Json<Value>> {
    Ok(Json(json!({
        "message": "Hello, World!",
        "request": body
    })))
}
