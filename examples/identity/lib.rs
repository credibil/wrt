//! # Identity Guest Module
//!
//! This module demonstrates the WASI Identity interface for obtaining
//! authentication credentials. It shows how to:
//! - Retrieve an identity provider from the host
//! - Request access tokens with specific scopes
//!
//! ## Use Cases
//!
//! - Authenticating to cloud APIs (Azure, AWS, GCP)
//! - Service-to-service authentication
//! - Obtaining tokens for downstream API calls
//!
//! ## Backend Flexibility
//!
//! The host determines the actual identity provider:
//! - Azure Managed Identity
//! - AWS IAM Roles
//! - Kubernetes Service Accounts
//! - Custom OAuth2 providers

#![cfg(target_arch = "wasm32")]

use anyhow::Context;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasi_identity::credentials::get_identity;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};
use wit_bindgen::block_on;

/// Azure Resource Manager scope for management operations.
/// Scopes define what resources the token can access.
const SCOPE: &str = "https://management.azure.com/.default";

/// HTTP handler struct.
struct Http;

/// Export the HTTP handler for the WASI runtime.
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    /// Routes incoming requests to the identity handler.
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::INFO)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router = Router::new().route("/", get(handler));
        wasi_http::serve(router, request).await
    }
}

/// Obtains an access token from the identity provider.
///
/// This handler demonstrates the WASI Identity workflow:
/// 1. Get an identity handle from the host (by name)
/// 2. Request an access token with specific scopes
/// 3. Use the token for authenticated API calls
///
/// ## Identity Names
///
/// The identity name ("identity") maps to host configuration that
/// determines which credential provider to use.
///
/// ## Scopes
///
/// Scopes define the permissions requested in the token.
/// Multiple scopes can be requested in a single call.
#[wasi_otel::instrument]
async fn handler() -> Result<Json<Value>> {
    // Get an identity handle from the host.
    // The name maps to a configured identity provider.
    let identity = block_on(get_identity("identity".to_string())).context("getting identity")?;

    // Request an access token with the specified scopes.
    // The token can be used for authenticated API calls.
    let scopes = vec![SCOPE.to_string()];
    let access_token = block_on(async move { identity.get_token(scopes).await })
        .context("getting access token")?;

    // In a real application, you would use this token in an Authorization header:
    // Authorization: Bearer {access_token.token}
    println!("access token: {}", access_token.token);

    Ok(Json(json!({
        "message": "Hello, World!"
    })))
}
