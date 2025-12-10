//! # KeyValue Store Guest Module
//!
//! This module demonstrates the WASI KeyValue interface. It shows how to:
//! - Open a named bucket (key-value namespace)
//! - Store and retrieve data by key
//! - Combine HTTP handling with storage operations
//!
//! ## Backend Agnostic
//!
//! The guest code here works unchanged with any WASI KeyValue backend:
//! - In-memory (this example's host)
//! - Redis (keyvalue-redis example)
//! - NATS JetStream (keyvalue-nats example)
//!
//! The host provides the concrete implementation at runtime.

#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasi_keyvalue::store;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

/// HTTP handler struct implementing the WASI HTTP Guest trait.
struct Http;

/// Export the HTTP handler for the WASI runtime.
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    /// Routes incoming HTTP requests to our handler.
    ///
    /// This example only exposes a POST endpoint that stores data
    /// in the key-value store.
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

/// Handles POST requests by storing the body in a key-value bucket.
///
/// This demonstrates the core WASI KeyValue operations:
/// 1. `store::open()` - Opens a named bucket (namespace)
/// 2. `bucket.set()` - Stores data under a key
/// 3. `bucket.get()` - Retrieves data by key
///
/// ## Parameters
/// - `body`: Raw bytes from the HTTP request body
///
/// ## Returns
/// A JSON response confirming the operation
///
/// ## Errors
/// Returns an error if bucket operations fail (e.g., bucket not found,
/// storage backend unavailable).
#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Result<Json<Value>> {
    // Open a named bucket. The bucket name is an application-level concept
    // that maps to whatever namespace/prefix the backend uses.
    // The `.context()` call adds a descriptive prefix to any errors.
    let bucket = store::open("credibil_bucket").context("opening bucket")?;

    // Store the request body under a fixed key.
    // In a real application, you'd likely derive the key from the request
    // (e.g., from a path parameter or request body field).
    bucket.set("my_key", &body).context("storing data")?;

    // Verify the data was stored by reading it back.
    // This demonstrates the get operation and is useful for debugging.
    let res = bucket.get("my_key");
    tracing::debug!("found val: {res:?}");

    Ok(Json(json!({
        "message": "Hello, World!"
    })))
}
