//! # Blobstore Guest Module
//!
//! This module demonstrates the WASI Blobstore interface for storing and
//! retrieving binary data (blobs). It shows how to:
//! - Create containers (namespaces for blobs)
//! - Write data using streaming `OutgoingValue`
//! - Read data using `IncomingValue`
//!
//! ## Backend Agnostic
//!
//! This guest code works with any WASI Blobstore backend:
//! - In-memory (this example's host)
//! - MongoDB (blobstore-mongodb example)
//! - NATS Object Store (blobstore-nats example)

#![cfg(target_arch = "wasm32")]

use anyhow::anyhow;
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
    /// Routes incoming HTTP requests to the blob storage handler.
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

/// Stores and retrieves data from the blobstore.
///
/// This handler demonstrates the complete blobstore workflow:
/// 1. Create an `OutgoingValue` to prepare data for writing
/// 2. Write data to the outgoing stream
/// 3. Create/open a container (namespace)
/// 4. Store the blob under a key
/// 5. Read it back using `IncomingValue`
///
/// ## Streaming API
///
/// The blobstore uses a streaming model for large data:
/// - `OutgoingValue`: Write stream for uploading blobs
/// - `IncomingValue`: Read stream for downloading blobs
///
/// This allows handling data larger than available memory.
#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Result<Json<Value>> {
    // Create an outgoing value to hold the data we want to store.
    // This is the write side of the streaming API.
    let outgoing = OutgoingValue::new_outgoing_value();

    // Get a write stream and write our data to it.
    let stream =
        outgoing.outgoing_value_write_body().map_err(|()| anyhow!("failed to create stream"))?;
    stream.blocking_write_and_flush(&body).map_err(|e| anyhow!("writing body: {e}"))?;

    // Create or open a container (like a bucket or namespace).
    // Containers group related blobs together.
    let container = blobstore::create_container("container")
        .map_err(|e| anyhow!("failed to create container: {e}"))?;

    // Write the blob to the container under the key "request".
    container.write_data("request", &outgoing).map_err(|e| anyhow!("failed to write data: {e}"))?;

    // Finalize the outgoing value (flushes any buffered data).
    OutgoingValue::finish(outgoing).map_err(|e| anyhow!("issue finishing: {e}"))?;

    // Read the blob back from the container.
    // Parameters: key, offset (0 = start), length (0 = read all).
    let incoming =
        container.get_data("request", 0, 0).map_err(|e| anyhow!("failed to read data: {e}"))?;

    // Consume the incoming stream synchronously to get the data.
    let data = IncomingValue::incoming_value_consume_sync(incoming)
        .map_err(|e| anyhow!("failed to create incoming value: {e}"))?;

    // Verify the round-trip was successful.
    assert_eq!(data, body);

    // Parse and return the data as JSON.
    let response =
        serde_json::from_slice::<Value>(&data).map_err(|e| anyhow!("deserializing data: {e}"))?;
    Ok(Json(response))
}
