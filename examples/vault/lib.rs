//! # Vault Guest Module
//!
//! This module demonstrates the WASI Vault interface for secure secret storage.
//! It shows how to:
//! - Open a vault "locker" (namespace for secrets)
//! - Store secrets securely
//! - Retrieve secrets by key
//!
//! ## Backend Agnostic
//!
//! This guest code works with any WASI Vault backend:
//! - In-memory (this example's host)
//! - Azure Key Vault (vault-azure example)
//! - HashiCorp Vault
//! - AWS Secrets Manager
//!
//! ## Security
//!
//! Secrets stored via WASI Vault are:
//! - Encrypted at rest (backend-dependent)
//! - Access-controlled by the host
//! - Never exposed in logs or traces

#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::Value;
use tracing::Level;
use wasi_http::Result;
use wasi_vault::vault;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    /// Routes incoming requests to the vault handler.
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", post(handler));
        wasi_http::serve(router, request).await
    }
}

/// Stores and retrieves a secret from the vault.
///
/// This handler demonstrates the WASI Vault workflow:
/// 1. Open a locker (named secret namespace)
/// 2. Store a secret under a key
/// 3. Retrieve the secret to verify storage
///
/// ## Lockers
///
/// A "locker" is a named namespace for secrets, similar to:
/// - Azure Key Vault vault
/// - HashiCorp Vault path
/// - AWS Secrets Manager prefix
///
/// ## Secret Versioning
///
/// Depending on the backend, secrets may be versioned.
/// The `get` operation returns the latest version.
#[wasi_otel::instrument]
async fn handler(body: Bytes) -> Result<Json<Value>> {
    // Open a locker (namespace) for storing secrets.
    // The locker name maps to host configuration.
    let locker = vault::open("credibil-locker").context("failed to open vault locker")?;

    // Store the secret under a key.
    // The data is securely stored by the backend.
    locker.set("secret-id", &body).context("issue setting secret")?;

    // Retrieve the secret to verify storage.
    // Returns Option<Vec<u8>> - None if not found.
    let secret = locker.get("secret-id").context("issue retriving secret")?;
    assert_eq!(secret.unwrap(), body);

    // Return the data (for demo purposes - don't expose real secrets!).
    let response = serde_json::from_slice::<Value>(&body).context("deserializing data")?;
    tracing::debug!("sending response: {response:?}");
    Ok(Json(response))
}
