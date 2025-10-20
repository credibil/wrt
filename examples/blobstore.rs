#![cfg(target_arch = "wasm32")]

use anyhow::{Context, anyhow};
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::Value;
use tracing::Level;
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::types::{IncomingRequest, ResponseOutparam};
use wasi_blobstore::blobstore;
use wasi_blobstore::types::{IncomingValue, OutgoingValue};
use wasi_http::Result;

struct HttpGuest;

impl Guest for HttpGuest {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let router = Router::new().route("/", post(handler));
        let out = wasi_http::serve(router, request);
        ResponseOutparam::set(response_out, out);
    }
}

#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Result<Json<Value>> {
    // write to blobstore
    let outgoing = OutgoingValue::new_outgoing_value();
    let stream =
        outgoing.outgoing_value_write_body().map_err(|_| anyhow!("failed create stream"))?;
    stream.blocking_write_and_flush(&body).context("writing body")?;

    let container = blobstore::create_container("container")
        .map_err(|e| anyhow!("failed to create container: {e}"))?;
    container.write_data("request", &outgoing).map_err(|e| anyhow!("failed to write data: {e}"))?;
    OutgoingValue::finish(outgoing).map_err(|e| anyhow!("issue finishing: {e}"))?;

    // read from blobstore
    let incoming =
        container.get_data("request", 0, 0).map_err(|e| anyhow!("failed to read data: {e}"))?;
    let data = IncomingValue::incoming_value_consume_sync(incoming)
        .map_err(|e| anyhow!("failed to create incoming value: {e}"))?;

    assert_eq!(data, body);

    let response = serde_json::from_slice::<Value>(&data).context("deserializing data")?;
    Ok(Json(response))
}

wasi::http::proxy::export!(HttpGuest);
