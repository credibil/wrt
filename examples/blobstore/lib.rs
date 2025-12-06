#![cfg(target_arch = "wasm32")]

use anyhow::{Context, anyhow};
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::Value;
use tracing::Level;
use wasi_blobstore::blobstore;
use wasi_blobstore::types::{IncomingValue, OutgoingValue};
use wasi_http::Result;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Result<Json<Value>> {
    // write to blobstore
    let outgoing = OutgoingValue::new_outgoing_value();
    let stream = outgoing.outgoing_value_write_body().context("failed create stream")?;
    stream.blocking_write_and_flush(&body).context("writing body")?;

    let container =
        blobstore::create_container("container").context("failed to create container")?;
    container.write_data("request", &outgoing).context("failed to write data")?;
    OutgoingValue::finish(outgoing).context("issue finishing")?;

    // read from blobstore
    let incoming = container.get_data("request", 0, 0).context("failed to read data")?;
    let data = IncomingValue::incoming_value_consume_sync(incoming)
        .context("failed to create incoming value")?;

    assert_eq!(data, body);

    let response = serde_json::from_slice::<Value>(&data).context("deserializing data")?;
    Ok(Json(response))
}
