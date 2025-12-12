//! # WebSockets Guest Module
//!
//! This module demonstrates the WASI WebSockets interface for real-time
//! bidirectional communication. It shows how to:
//! - Access a WebSocket server managed by the host
//! - Query connected peers
//! - Send messages to specific peers
//! - Implement health checks
//!
//! ## Architecture
//!
//! The host manages the WebSocket connections and exposes them to the guest
//! via the WASI WebSockets interface. The guest can:
//! - List connected peers
//! - Send messages to peers
//! - Check server health
//!
//! ## Endpoints
//!
//! - `GET /health`: Check WebSocket server health
//! - `POST /socket`: Send a message to all connected peers

#![cfg(target_arch = "wasm32")]

use std::println;

use anyhow::anyhow;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use wasi_http::Result;
use wasi_websockets::store;
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct HttpGuest;
wasip3::http::proxy::export!(HttpGuest);

impl Guest for HttpGuest {
    /// Routes HTTP requests to WebSocket management endpoints.
    ///
    /// Note: WebSocket upgrade requests are handled by the host, not this handler.
    /// This handler provides HTTP endpoints for managing WebSocket peers.
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        let router =
            Router::new().route("/health", get(get_handler)).route("/socket", post(post_handler));
        wasi_http::serve(router, request).await
    }
}

/// Health check endpoint for the WebSocket server.
///
/// Returns the server's health status, which can be used for:
/// - Load balancer health checks
/// - Monitoring systems
/// - Debugging connection issues
#[axum::debug_handler]
async fn get_handler() -> Result<Json<Value>> {
    // Get the WebSocket server handle from the host.
    let server = store::get_server().map_err(|e| anyhow!("getting websocket server: {e}"))?;

    // Query the server's health status.
    let message = server.health_check().unwrap_or_else(|_| "Service is unhealthy".to_string());

    Ok(Json(json!({
        "message": message
    })))
}

/// Sends a message to all connected WebSocket peers.
///
/// This demonstrates the broadcast pattern:
/// 1. Get the WebSocket server handle
/// 2. List all connected peers
/// 3. Send the message to each peer
///
/// ## Parameters
/// - `body`: The message to broadcast (as a string)
///
/// ## Returns
/// Confirmation that the message was received for processing
#[axum::debug_handler]
async fn post_handler(body: String) -> Result<Json<Value>> {
    // Get the WebSocket server handle.
    let server = store::get_server().map_err(|e| anyhow!("getting websocket server: {e}"))?;

    // Get list of all connected peers.
    let peers = server.get_peers();
    let client_peers = match peers {
        Ok(p) => p,
        Err(e) => {
            println!("Error retrieving websocket peers: {e}");
            return Err(anyhow!("error retrieving websocket peers").into());
        }
    };

    // Extract peer addresses for the broadcast.
    let recipients: Vec<String> = client_peers.iter().map(|p| p.address.clone()).collect();

    // Send the message to all peers.
    if let Err(e) = server.send_peers(&body, &recipients) {
        println!("Error sending websocket message: {e}");
    }

    Ok(Json(json!({
        "message": "message received"
    })))
}
